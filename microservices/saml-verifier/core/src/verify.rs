//! Verification of one SAML response for one connection.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use chrono::{DateTime, Utc};
use samael::crypto::AllowedSignatureAlgorithm;
use samael::metadata::EntityDescriptor;
use samael::schema::Assertion;
use samael::service_provider::{Error as SpError, ServiceProvider, ServiceProviderBuilder};

use crate::precheck::precheck;
use crate::Refusal;

/// Largest decoded response accepted (ADR-014 §3).
pub const MAX_RESPONSE_BYTES: usize = 256 * 1024;
/// Largest clock skew a caller may ask for.
pub const MAX_CLOCK_SKEW_SECS: i64 = 300;
const STATUS_SUCCESS: &str = "urn:oasis:names:tc:SAML:2.0:status:Success";
const ALLOWED_ALGORITHMS: &[AllowedSignatureAlgorithm] = &[
    AllowedSignatureAlgorithm::RsaSha256,
    AllowedSignatureAlgorithm::RsaSha384,
    AllowedSignatureAlgorithm::RsaSha512,
    AllowedSignatureAlgorithm::EcdsaSha256,
    AllowedSignatureAlgorithm::EcdsaSha384,
];

/// Everything a verification decision needs, from identity-login-service.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifyRequest {
    /// The `SAMLResponse` form value as posted to the ACS (base64).
    pub saml_response: String,
    /// The connection's IdP entity ID.
    pub idp_entity_id: String,
    /// The connection's pinned IdP signing certificates (base64 DER), at least one.
    pub idp_certificates: Vec<String>,
    /// The connection's SP entity ID (the audience).
    pub sp_entity_id: String,
    /// The connection's ACS URL (destination and recipient).
    pub acs_url: String,
    /// ID of the one pending AuthnRequest (IdP-initiated login is never accepted).
    pub expected_request_id: String,
    /// Allowed clock skew in seconds (default 120, at most 300).
    #[serde(default)]
    pub max_clock_skew_secs: Option<i64>,
}

/// The verified, normalised result.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct VerifiedAssertion {
    /// Assertion ID (replay protection in identity-login, until `not_on_or_after`).
    pub assertion_id: String,
    /// IdP entity ID that issued it.
    pub issuer: String,
    /// The subject's NameID.
    pub name_id: String,
    /// NameID format, if given.
    pub name_id_format: Option<String>,
    /// Attributes by name, values in document order.
    pub attributes: BTreeMap<String, Vec<String>>,
    /// When the user authenticated at the IdP.
    pub authn_instant: Option<DateTime<Utc>>,
    /// IdP session index.
    pub session_index: Option<String>,
    /// IdP session end, if given.
    pub session_not_on_or_after: Option<DateTime<Utc>>,
    /// End of the assertion's validity (`Conditions/@NotOnOrAfter`).
    pub not_on_or_after: DateTime<Utc>,
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Minimal IdP metadata carrying the entity ID and the pinned certificates, as samael takes them.
fn idp_metadata(entity_id: &str, certificates: &[String]) -> Result<EntityDescriptor, Refusal> {
    let mut keys = String::new();
    for cert in certificates {
        let compact: String = cert.split_ascii_whitespace().collect();
        let der = STANDARD
            .decode(&compact)
            .map_err(|_| Refusal::NoCertificate)?;
        if der.is_empty() {
            return Err(Refusal::NoCertificate);
        }
        let _ = write!(
            keys,
            r#"<md:KeyDescriptor use="signing"><ds:KeyInfo><ds:X509Data><ds:X509Certificate>{compact}</ds:X509Certificate></ds:X509Data></ds:KeyInfo></md:KeyDescriptor>"#
        );
    }
    let xml = format!(
        r#"<md:EntityDescriptor xmlns:md="urn:oasis:names:tc:SAML:2.0:metadata" xmlns:ds="http://www.w3.org/2000/09/xmldsig#" entityID="{}"><md:IDPSSODescriptor protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol">{keys}<md:SingleSignOnService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect" Location="https://idp.invalid/sso"/></md:IDPSSODescriptor></md:EntityDescriptor>"#,
        xml_escape(entity_id)
    );
    xml.parse::<EntityDescriptor>()
        .map_err(|_| Refusal::BadRequest)
}

fn map_sp_error(e: &SpError) -> Refusal {
    match e {
        SpError::FailedToValidateSignature => Refusal::Signature,
        SpError::ResponseIssuerMismatch { .. } | SpError::AssertionIssuerMismatch { .. } => {
            Refusal::Issuer
        }
        SpError::ResponseInResponseToInvalid { .. }
        | SpError::AssertionInResponseToInvalid { .. } => Refusal::InResponseTo,
        SpError::DestinationValidationError { .. } | SpError::AssertionRecipientMismatch { .. } => {
            Refusal::Destination
        }
        SpError::AssertionConditionAudienceRestrictionFailed { .. } => Refusal::Audience,
        SpError::ResponseExpired { .. }
        | SpError::AssertionExpired { .. }
        | SpError::AssertionConditionExpired { .. }
        | SpError::AssertionConditionExpiredBefore { .. }
        | SpError::AssertionSubjectConfirmationExpired { .. }
        | SpError::AssertionSubjectConfirmationExpiredBefore { .. } => Refusal::Time,
        SpError::ResponseBadStatusCode { .. } => Refusal::Status,
        _ => Refusal::Invalid,
    }
}

fn post_check(
    assertion: &Assertion,
    request: &VerifyRequest,
    raw_assertion_id: &str,
) -> Result<VerifiedAssertion, Refusal> {
    if assertion.id != raw_assertion_id {
        return Err(Refusal::AssertionIdMismatch);
    }
    let issuer = assertion.issuer.value.clone().unwrap_or_default();
    if issuer != request.idp_entity_id {
        return Err(Refusal::Issuer);
    }
    let conditions = assertion.conditions.as_ref().ok_or(Refusal::Audience)?;
    let audience_ok = conditions.audience_restrictions.as_ref().is_some_and(|rs| {
        !rs.is_empty()
            && rs
                .iter()
                .all(|r| r.audience.contains(&request.sp_entity_id))
    });
    if !audience_ok {
        return Err(Refusal::Audience);
    }
    let not_on_or_after = conditions.not_on_or_after.ok_or(Refusal::Time)?;
    let authn = assertion
        .authn_statements
        .as_ref()
        .and_then(|s| s.first())
        .ok_or(Refusal::NoAuthnStatement)?;
    let name_id = assertion
        .subject
        .as_ref()
        .and_then(|s| s.name_id.as_ref())
        .ok_or(Refusal::NameId)?;
    let value = name_id.value.trim();
    if value.is_empty() {
        return Err(Refusal::NameId);
    }
    let mut attributes: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for statement in assertion.attribute_statements.iter().flatten() {
        for attribute in &statement.attributes {
            let Some(name) = attribute.name.clone() else {
                continue;
            };
            let values = attributes.entry(name).or_default();
            values.extend(attribute.values.iter().filter_map(|v| v.value.clone()));
        }
    }
    Ok(VerifiedAssertion {
        assertion_id: assertion.id.clone(),
        issuer,
        name_id: value.to_string(),
        name_id_format: name_id.format.clone(),
        attributes,
        authn_instant: authn.authn_instant,
        session_index: authn.session_index.clone(),
        session_not_on_or_after: authn.session_not_on_or_after,
        not_on_or_after,
    })
}

/// Verify one response.
///
/// # Errors
///
/// The first [`Refusal`] met: pre-checks, then samael and libxmlsec1, then the post-checks.
pub fn verify(request: &VerifyRequest) -> Result<VerifiedAssertion, Refusal> {
    if request.idp_entity_id.is_empty()
        || request.sp_entity_id.is_empty()
        || request.acs_url.is_empty()
        || request.expected_request_id.is_empty()
    {
        return Err(Refusal::BadRequest);
    }
    let skew = request.max_clock_skew_secs.unwrap_or(120);
    if !(0..=MAX_CLOCK_SKEW_SECS).contains(&skew) {
        return Err(Refusal::BadRequest);
    }
    if request.idp_certificates.is_empty() {
        // samael verifies nothing when it has no certificates: never call it without one (ADR-014 §1).
        return Err(Refusal::NoCertificate);
    }
    if request.saml_response.len() > MAX_RESPONSE_BYTES * 4 / 3 + 1024 {
        return Err(Refusal::TooLarge);
    }
    let compact: String = request.saml_response.split_ascii_whitespace().collect();
    let bytes = STANDARD.decode(compact).map_err(|_| Refusal::BadBase64)?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(Refusal::TooLarge);
    }
    let xml = String::from_utf8(bytes).map_err(|_| Refusal::NotUtf8)?;

    let shape = precheck(&xml)?;
    if shape.status.as_deref() != Some(STATUS_SUCCESS) {
        return Err(Refusal::Status);
    }
    if shape
        .destination
        .as_deref()
        .is_some_and(|d| d != request.acs_url)
    {
        return Err(Refusal::Destination);
    }
    if shape
        .in_response_to
        .as_deref()
        .is_some_and(|id| id != request.expected_request_id)
    {
        return Err(Refusal::InResponseTo);
    }

    let metadata = idp_metadata(&request.idp_entity_id, &request.idp_certificates)?;
    let sp: ServiceProvider = ServiceProviderBuilder::default()
        .entity_id(Some(request.sp_entity_id.clone()))
        .acs_url(Some(request.acs_url.clone()))
        .metadata_url(None::<String>)
        .slo_url(None::<String>)
        .idp_metadata(metadata)
        .allow_idp_initiated(false)
        .max_clock_skew(chrono::Duration::seconds(skew))
        .max_issue_delay(chrono::Duration::seconds(90 + skew))
        .allowed_signature_algorithms(Some(ALLOWED_ALGORITHMS.to_vec()))
        .build()
        .map_err(|_| Refusal::BadRequest)?;
    match sp.idp_signing_certs() {
        Ok(Some(certs)) if certs.len() == request.idp_certificates.len() => {}
        _ => return Err(Refusal::NoCertificate),
    }
    let assertion = sp
        .parse_xml_response(&xml, Some(&[request.expected_request_id.as_str()]))
        .map_err(|e| map_sp_error(&e))?;
    post_check(&assertion, request, &shape.assertion_id)
}
