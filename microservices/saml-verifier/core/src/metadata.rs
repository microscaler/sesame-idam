//! IdP metadata parsing (DESIGN-saml-sp.md §7): what a connection stores from an IdP's `EntityDescriptor`.
//!
//! The metadata is configuration an org admin supplied (uploaded, or fetched by org-mgmt from an HTTPS URL); its
//! own signature is not verified here (later work). It is still hostile-input-shaped XML, so it gets the same
//! parser guards as responses: size limit, no DOCTYPE or entities, no processing instructions.

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use chrono::{DateTime, Utc};
use quick_xml::events::Event;
use quick_xml::Reader;
use samael::metadata::EntityDescriptor;

use crate::Refusal;

/// Largest metadata document accepted.
pub const MAX_METADATA_BYTES: usize = 1024 * 1024;
const REDIRECT_BINDING: &str = "urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect";

/// A signing certificate from the metadata.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SigningCert {
    /// Certificate, base64 DER.
    pub der: String,
    /// End of validity.
    pub not_after: DateTime<Utc>,
}

/// What a connection keeps from the IdP's metadata.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct IdpMetadata {
    /// IdP entity ID.
    pub entity_id: String,
    /// `SingleSignOnService` location for the HTTP-Redirect binding (https).
    pub sso_redirect_url: String,
    /// Signing certificates (all of them, for rollover).
    pub signing_certs: Vec<SigningCert>,
    /// Supported NameID formats.
    pub name_id_formats: Vec<String>,
}

fn guard(xml: &str) -> Result<(), Refusal> {
    let mut reader = Reader::from_str(xml);
    loop {
        match reader.read_event().map_err(|_| Refusal::Malformed)? {
            Event::Eof => return Ok(()),
            Event::DocType(_) => return Err(Refusal::Doctype),
            Event::PI(_) => return Err(Refusal::ProcessingInstruction),
            Event::GeneralRef(r) => {
                // Only the five predefined entities and character references.
                let name = String::from_utf8_lossy(r.as_ref()).into_owned();
                if !matches!(name.as_str(), "amp" | "lt" | "gt" | "quot" | "apos")
                    && !name.starts_with('#')
                {
                    return Err(Refusal::Doctype);
                }
            }
            _ => {}
        }
    }
}

fn not_after(der: &[u8]) -> Result<DateTime<Utc>, Refusal> {
    let cert = openssl::x509::X509::from_der(der).map_err(|_| Refusal::NoCertificate)?;
    let epoch = openssl::asn1::Asn1Time::from_unix(0).map_err(|_| Refusal::NoCertificate)?;
    let diff = epoch
        .diff(cert.not_after())
        .map_err(|_| Refusal::NoCertificate)?;
    let secs = i64::from(diff.days) * 86_400 + i64::from(diff.secs);
    DateTime::from_timestamp(secs, 0).ok_or(Refusal::NoCertificate)
}

/// Parse IdP metadata.
///
/// # Errors
///
/// [`Refusal::TooLarge`], parser-guard refusals, [`Refusal::Metadata`] when there is no single IdP with an
/// HTTPS HTTP-Redirect sign-on service, [`Refusal::NoCertificate`] when no usable signing certificate is present.
pub fn parse_metadata(xml: &str) -> Result<IdpMetadata, Refusal> {
    if xml.len() > MAX_METADATA_BYTES {
        return Err(Refusal::TooLarge);
    }
    guard(xml)?;
    let entity: EntityDescriptor = xml.parse().map_err(|_| Refusal::Metadata)?;
    let entity_id = entity
        .entity_id
        .clone()
        .filter(|e| !e.trim().is_empty())
        .ok_or(Refusal::Metadata)?;
    let Some([idp]) = entity.idp_sso_descriptors.as_deref() else {
        return Err(Refusal::Metadata);
    };
    let sso_redirect_url = idp
        .single_sign_on_services
        .iter()
        .find(|s| s.binding == REDIRECT_BINDING && s.location.starts_with("https://"))
        .map(|s| s.location.clone())
        .ok_or(Refusal::Metadata)?;

    let signing: Vec<_> = idp
        .key_descriptors
        .iter()
        .filter(|k| k.key_use.as_deref() == Some("signing"))
        .collect();
    let candidates = if signing.is_empty() {
        idp.key_descriptors
            .iter()
            .filter(|k| k.key_use.as_deref().is_none_or(str::is_empty))
            .collect()
    } else {
        signing
    };
    let mut signing_certs = Vec::new();
    for key in candidates {
        for cert in key
            .key_info
            .x509_data
            .iter()
            .flat_map(|d| d.certificates.iter())
        {
            let compact: String = cert.split_ascii_whitespace().collect();
            let der = STANDARD
                .decode(&compact)
                .map_err(|_| Refusal::NoCertificate)?;
            let not_after = not_after(&der)?;
            if !signing_certs.iter().any(|c: &SigningCert| c.der == compact) {
                signing_certs.push(SigningCert {
                    der: compact,
                    not_after,
                });
            }
        }
    }
    if signing_certs.is_empty() {
        return Err(Refusal::NoCertificate);
    }
    Ok(IdpMetadata {
        entity_id,
        sso_redirect_url,
        signing_certs,
        name_id_formats: idp.name_id_formats.clone(),
    })
}
