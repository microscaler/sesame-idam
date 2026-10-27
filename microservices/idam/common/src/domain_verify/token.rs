//! Challenge tokens and TXT records.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use uuid::Uuid;

use super::keyring::{EnrollKeyring, KeyringError, MAC_LEN};
use super::{Domain, DomainError};

/// Leaf label of the challenge name: `_sesame-challenge.<domain>`.
pub const CHALLENGE_LABEL: &str = "_sesame-challenge";
/// Prefix of every Sesame TXT string; other strings at the same name are ignored.
pub const RECORD_PREFIX: &str = "sesame-domain-verify=";
/// Record and PRF version.
pub const TOKEN_VERSION: &str = "v1";
/// Purpose of an organisation email-domain claim.
pub const ORG_DOMAIN_PURPOSE: &str = "sesame-org-domain";
/// Purpose of a tenant issuer-domain claim (ADR-004, ADR-007).
pub const TENANT_ISSUER_PURPOSE: &str = "sesame-domain-verify";

/// HMAC-SHA256 output in base64url without padding.
const TOKEN_LEN: usize = 43;
/// Longest tenant identifier (`tenants.id` is `VARCHAR(64)`).
const MAX_TENANT_ID: usize = 64;

/// What an organisation domain claim covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    /// Addresses at the domain itself only (default).
    Exact,
    /// Addresses at the domain and any subdomain of it.
    Subdomains,
}

impl Scope {
    /// Stored and PRF form: `exact` or `subdomains`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Subdomains => "subdomains",
        }
    }

    /// Parse the stored form.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "exact" => Some(Self::Exact),
            "subdomains" => Some(Self::Subdomains),
            _ => None,
        }
    }

    /// Whether a claim on `claimed` with this scope covers an address at `email_domain`.
    #[must_use]
    pub fn covers(self, claimed: &Domain, email_domain: &Domain) -> bool {
        match self {
            Self::Exact => claimed == email_domain,
            Self::Subdomains => email_domain.is_same_or_under(claimed),
        }
    }
}

/// Who a claim is for, and which claim. Built only through the validating constructors.
///
/// `claim_id` is the claim's own row id: every new claim, even for the same organisation and domain, gets a new
/// token, so re-claiming after a revocation needs a deliberate new DNS change.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ClaimSubject {
    /// An organisation's email domain.
    Organisation {
        /// Tenant identifier.
        tenant_id: String,
        /// Organisation identifier.
        org_id: Uuid,
        /// The claim's id.
        claim_id: Uuid,
        /// Exact domain or domain and subdomains.
        scope: Scope,
    },
    /// A tenant's issuer domain.
    TenantIssuer {
        /// Tenant identifier.
        tenant_id: String,
        /// The enrolment's id.
        claim_id: Uuid,
    },
}

fn check_tenant_id(tenant_id: &str) -> Result<String, DomainError> {
    if tenant_id.is_empty() {
        return Err(DomainError::InvalidSubject("empty tenant id"));
    }
    if tenant_id.len() > MAX_TENANT_ID {
        return Err(DomainError::InvalidSubject("tenant id too long"));
    }
    if tenant_id.bytes().any(|b| b == 0) {
        return Err(DomainError::InvalidSubject("tenant id contains NUL"));
    }
    Ok(tenant_id.to_string())
}

impl ClaimSubject {
    /// An organisation email-domain claim.
    ///
    /// # Errors
    ///
    /// [`DomainError::InvalidSubject`] for an empty, over-long or NUL-containing tenant id.
    pub fn organisation(
        tenant_id: &str,
        org_id: Uuid,
        claim_id: Uuid,
        scope: Scope,
    ) -> Result<Self, DomainError> {
        Ok(Self::Organisation {
            tenant_id: check_tenant_id(tenant_id)?,
            org_id,
            claim_id,
            scope,
        })
    }

    /// A tenant issuer-domain claim.
    ///
    /// # Errors
    ///
    /// [`DomainError::InvalidSubject`] for an empty, over-long or NUL-containing tenant id.
    pub fn tenant_issuer(tenant_id: &str, claim_id: Uuid) -> Result<Self, DomainError> {
        Ok(Self::TenantIssuer {
            tenant_id: check_tenant_id(tenant_id)?,
            claim_id,
        })
    }

    /// The MAC input: fields separated by 0x00, ending with the purpose and version.
    ///
    /// ```text
    /// organisation:  tenant_id 0 org_id 0 claim_id 0 fqdn 0 scope 0 "sesame-org-domain" 0 "v1"
    /// tenant issuer: tenant_id 0 claim_id 0 fqdn 0 "sesame-domain-verify" 0 "v1"
    /// ```
    ///
    /// Identifiers are lower-case hyphenated UUIDs. No field can contain 0x00 (tenant ids are checked, the
    /// others are UUIDs, A-labels and constants), so the encoding is unambiguous.
    #[must_use]
    pub fn mac_input(&self, domain: &Domain) -> Vec<u8> {
        let fields: Vec<String> = match self {
            Self::Organisation {
                tenant_id,
                org_id,
                claim_id,
                scope,
            } => vec![
                tenant_id.clone(),
                org_id.hyphenated().to_string(),
                claim_id.hyphenated().to_string(),
                domain.as_str().to_string(),
                scope.as_str().to_string(),
                ORG_DOMAIN_PURPOSE.to_string(),
                TOKEN_VERSION.to_string(),
            ],
            Self::TenantIssuer {
                tenant_id,
                claim_id,
            } => vec![
                tenant_id.clone(),
                claim_id.hyphenated().to_string(),
                domain.as_str().to_string(),
                TENANT_ISSUER_PURPOSE.to_string(),
                TOKEN_VERSION.to_string(),
            ],
        };
        fields.join("\0").into_bytes()
    }
}

/// `_sesame-challenge.<domain>`: where the owner publishes the record.
#[must_use]
pub fn challenge_name(domain: &Domain) -> String {
    format!("{CHALLENGE_LABEL}.{domain}")
}

/// The token for `subject` and `domain` under a raw key (43 characters of base64url).
///
/// For test vectors and tools only; services derive tokens through an [`EnrollKeyring`] so the key can stay
/// in an HSM.
///
/// # Panics
///
/// Never: HMAC accepts keys of any length.
#[must_use]
pub fn derive_token(key: &[u8], subject: &ClaimSubject, domain: &Domain) -> String {
    // HMAC accepts keys of any length; `new_from_slice` cannot fail for it.
    #[allow(clippy::expect_used)]
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC takes any key length");
    mac.update(&subject.mac_input(domain));
    URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
}

/// The token inside a Sesame TXT string, or `None` when the string is not a well-formed `v1` record.
///
/// Strict: the string must be exactly `sesame-domain-verify=v1;t=<43 base64url characters>`. Strings of other
/// versions, with any whitespace or extra fields, are ignored (never an error), so unrelated and future
/// records at the same name do no harm. `value` is one TXT record with its character-strings concatenated.
#[must_use]
pub fn parse_record(value: &str) -> Option<&str> {
    let rest = value.strip_prefix(RECORD_PREFIX)?;
    let (version, rest) = rest.split_once(';')?;
    if version != TOKEN_VERSION {
        return None;
    }
    let token = rest.strip_prefix("t=")?;
    let well_formed = token.len() == TOKEN_LEN
        && token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    well_formed.then_some(token)
}

/// The record to publish for a claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxtRecord {
    /// Owner name: `_sesame-challenge.<domain>`.
    pub name: String,
    /// TXT value: `sesame-domain-verify=v1;t=<token>`.
    pub value: String,
    /// Key epoch the token was derived under (stored with the claim).
    pub epoch: u32,
}

/// The record to publish for `subject` and `domain`, under the keyring's current epoch.
///
/// # Errors
///
/// [`KeyringError`] when the key cannot be used (for example the KMS is unreachable).
pub fn expected_record(
    keyring: &dyn EnrollKeyring,
    subject: &ClaimSubject,
    domain: &Domain,
) -> Result<TxtRecord, KeyringError> {
    let epoch = keyring.current_epoch();
    let tag = keyring.mac(epoch, &subject.mac_input(domain))?;
    Ok(TxtRecord {
        name: challenge_name(domain),
        value: format!(
            "{RECORD_PREFIX}{TOKEN_VERSION};t={}",
            URL_SAFE_NO_PAD.encode(tag)
        ),
        epoch,
    })
}

/// Constant-time equality of two MACs.
fn tags_equal(a: &[u8; MAC_LEN], b: &[u8]) -> bool {
    if b.len() != MAC_LEN {
        return false;
    }
    let diff = a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y));
    std::hint::black_box(diff) == 0
}

/// Check the TXT values found at the challenge name. `Ok(Some(epoch))` names the epoch whose key matched;
/// `Ok(None)` means no value matched.
///
/// Each value is one TXT record (its character-strings concatenated). Non-Sesame and other-version strings are
/// skipped. The expected tag is computed once per accepted epoch (current first) and every presented token is
/// compared with every expected tag in constant time, with no early exit.
///
/// # Errors
///
/// [`KeyringError`] when the key cannot be used. Callers must treat that as "could not check", never as
/// "record missing": a KMS outage must not move verified domains into grace.
pub fn verify_record<I, S>(
    keyring: &dyn EnrollKeyring,
    subject: &ClaimSubject,
    domain: &Domain,
    txt_values: I,
) -> Result<Option<u32>, KeyringError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let message = subject.mac_input(domain);
    let mut expected = Vec::new();
    for epoch in keyring.accepted_epochs() {
        expected.push((epoch, keyring.mac(epoch, &message)?));
    }
    let mut matched = None;
    for value in txt_values {
        let Some(token) = parse_record(value.as_ref()) else {
            continue;
        };
        let Ok(presented) = URL_SAFE_NO_PAD.decode(token) else {
            continue;
        };
        for (epoch, tag) in &expected {
            if tags_equal(tag, &presented) && matched.is_none() {
                matched = Some(*epoch);
            }
        }
    }
    Ok(matched)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain_verify::{normalize_domain, LocalKeyring};

    const ORG: Uuid = Uuid::from_u128(0xb200_0002_0002_4000_8000_0000_0000_0002);
    const CLAIM: Uuid = Uuid::from_u128(0x0193_a4b2_7c1e_7d3a_9f00_1234_5678_90ab);

    fn key(byte: u8) -> Vec<u8> {
        vec![byte; 32]
    }

    fn domain(d: &str) -> Domain {
        normalize_domain(d).expect("valid")
    }

    fn org(scope: Scope) -> ClaimSubject {
        ClaimSubject::organisation("acme", ORG, CLAIM, scope).expect("valid")
    }

    fn ring(current: (u32, u8), previous: Option<(u32, u8)>) -> LocalKeyring {
        LocalKeyring::new(
            (current.0, key(current.1)),
            previous.map(|(e, b)| (e, key(b))),
        )
        .expect("key")
    }

    #[test]
    fn record_shape() {
        let k = ring((7, 1), None);
        let rec = expected_record(&k, &org(Scope::Exact), &domain("BigBank.com")).expect("mac");
        assert_eq!(rec.name, "_sesame-challenge.bigbank.com");
        assert!(rec.value.starts_with("sesame-domain-verify=v1;t="));
        assert_eq!(rec.value.len(), RECORD_PREFIX.len() + 5 + TOKEN_LEN);
        assert_eq!(rec.epoch, 7);
        assert!(parse_record(&rec.value).is_some());
    }

    #[test]
    fn keyring_and_raw_key_agree() {
        let k = ring((1, 9), None);
        let d = domain("bigbank.com");
        let s = org(Scope::Exact);
        let rec = expected_record(&k, &s, &d).expect("mac");
        assert_eq!(
            parse_record(&rec.value),
            Some(derive_token(&key(9), &s, &d).as_str())
        );
    }

    #[test]
    fn verify_accepts_current_and_previous_only() {
        let d = domain("bigbank.com");
        let s = org(Scope::Exact);
        let old = ring((1, 1), None);
        let rotated = ring((2, 2), Some((1, 1)));
        let retired = ring((3, 3), Some((2, 2)));
        let published = expected_record(&old, &s, &d).expect("mac").value;
        assert_eq!(verify_record(&rotated, &s, &d, [&published]), Ok(Some(1)));
        assert_eq!(verify_record(&retired, &s, &d, [&published]), Ok(None));
        let fresh = expected_record(&rotated, &s, &d).expect("mac").value;
        assert_eq!(verify_record(&rotated, &s, &d, [&fresh]), Ok(Some(2)));
    }

    #[test]
    fn verify_finds_its_record_among_others() {
        let d = domain("bigbank.com");
        let s = org(Scope::Exact);
        let k = ring((1, 9), None);
        let other =
            ClaimSubject::organisation("other-tenant", ORG, CLAIM, Scope::Exact).expect("valid");
        let values = vec![
            "v=spf1 -all".to_string(),
            expected_record(&k, &other, &d).expect("mac").value,
            "sesame-domain-verify=v2;t=whatever".to_string(),
            expected_record(&k, &s, &d).expect("mac").value,
        ];
        assert_eq!(verify_record(&k, &s, &d, &values), Ok(Some(1)));
        assert_eq!(verify_record(&k, &s, &d, &values[..3]), Ok(None));
    }

    #[test]
    fn malformed_records_never_match() {
        let d = domain("bigbank.com");
        let s = org(Scope::Exact);
        let k = ring((1, 5), None);
        let good = expected_record(&k, &s, &d).expect("mac").value;
        let token = parse_record(&good).expect("well formed").to_string();
        for bad in [
            format!(" {good}"),
            format!("{good} "),
            format!("sesame-domain-verify=v1; t={token}"),
            format!("sesame-domain-verify=v1;t= {token}"),
            format!("sesame-domain-verify=V1;t={token}"),
            format!("Sesame-domain-verify=v1;t={token}"),
            format!("x-sesame-domain-verify=v1;t={token}"),
            format!("sesame-domain-verify=v2;t={token}"),
            format!("sesame-domain-verify=v1;t={token};x=1"),
            format!("sesame-domain-verify=v1;t={token}="),
            format!("sesame-domain-verify=v1;t={}", &token[..42]),
            token.clone(),
        ] {
            assert_eq!(verify_record(&k, &s, &d, [&bad]), Ok(None), "{bad:?}");
        }
    }

    #[test]
    fn binding_to_subject_claim_domain_and_scope() {
        let k = ring((1, 4), None);
        let d = domain("bigbank.com");
        let published = expected_record(&k, &org(Scope::Exact), &d)
            .expect("mac")
            .value;
        let check = |s: &ClaimSubject, dom: &Domain| verify_record(&k, s, dom, [&published]);
        assert_eq!(check(&org(Scope::Subdomains), &d), Ok(None));
        assert_eq!(
            check(&org(Scope::Exact), &domain("eu.bigbank.com")),
            Ok(None)
        );
        let issuer = ClaimSubject::tenant_issuer("acme", CLAIM).expect("valid");
        assert_eq!(check(&issuer, &d), Ok(None));
        let other_org = ClaimSubject::organisation("acme", Uuid::from_u128(1), CLAIM, Scope::Exact)
            .expect("valid");
        assert_eq!(check(&other_org, &d), Ok(None));
        let reclaimed = ClaimSubject::organisation("acme", ORG, Uuid::from_u128(2), Scope::Exact)
            .expect("valid");
        assert_eq!(
            check(&reclaimed, &d),
            Ok(None),
            "a new claim needs a new record"
        );
    }

    #[test]
    fn keyring_failure_is_an_error_not_a_miss() {
        struct Down;
        impl EnrollKeyring for Down {
            fn current_epoch(&self) -> u32 {
                1
            }
            fn accepted_epochs(&self) -> Vec<u32> {
                vec![1]
            }
            fn mac(&self, _: u32, _: &[u8]) -> Result<[u8; MAC_LEN], KeyringError> {
                Err(KeyringError::Unavailable("kms down".to_string()))
            }
        }
        let d = domain("bigbank.com");
        assert!(verify_record(&Down, &org(Scope::Exact), &d, ["x"]).is_err());
        assert!(expected_record(&Down, &org(Scope::Exact), &d).is_err());
    }

    #[test]
    fn subjects_validate_tenant_id() {
        assert!(ClaimSubject::tenant_issuer("", CLAIM).is_err());
        assert!(ClaimSubject::tenant_issuer("a\0b", CLAIM).is_err());
        assert!(ClaimSubject::tenant_issuer(&"t".repeat(65), CLAIM).is_err());
        assert!(ClaimSubject::tenant_issuer(&"t".repeat(64), CLAIM).is_ok());
    }

    #[test]
    fn scope_coverage() {
        let claimed = domain("bigbank.com");
        assert!(Scope::Exact.covers(&claimed, &domain("bigbank.com")));
        assert!(!Scope::Exact.covers(&claimed, &domain("eu.bigbank.com")));
        assert!(Scope::Subdomains.covers(&claimed, &domain("eu.bigbank.com")));
        assert!(!Scope::Subdomains.covers(&claimed, &domain("notbigbank.com")));
        assert_eq!(Scope::parse("subdomains"), Some(Scope::Subdomains));
        assert_eq!(Scope::parse("Exact"), None);
    }
}
