//! Challenge tokens, TXT records and the enrolment key.

use std::fmt;
use std::path::Path;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

use super::{Domain, DomainError};

type HmacSha256 = Hmac<Sha256>;

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
const TOKEN_BYTES: usize = 32;
/// Minimum enrolment key length (256 bits).
const MIN_KEY_BYTES: usize = 32;
/// Longest tenant identifier (`tenants.id` is `VARCHAR(64)`).
const MAX_TENANT_ID: usize = 64;
/// Largest key file accepted.
const MAX_KEY_FILE: u64 = 16 * 1024;

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

/// Who a claim is for. Built only through the validating constructors.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ClaimSubject {
    /// An organisation's email domain.
    Organisation {
        /// Tenant identifier.
        tenant_id: String,
        /// Organisation identifier.
        org_id: Uuid,
        /// Exact domain or domain and subdomains.
        scope: Scope,
    },
    /// A tenant's issuer domain.
    TenantIssuer {
        /// Tenant identifier.
        tenant_id: String,
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
    pub fn organisation(tenant_id: &str, org_id: Uuid, scope: Scope) -> Result<Self, DomainError> {
        Ok(Self::Organisation {
            tenant_id: check_tenant_id(tenant_id)?,
            org_id,
            scope,
        })
    }

    /// A tenant issuer-domain claim.
    ///
    /// # Errors
    ///
    /// [`DomainError::InvalidSubject`] for an empty, over-long or NUL-containing tenant id.
    pub fn tenant_issuer(tenant_id: &str) -> Result<Self, DomainError> {
        Ok(Self::TenantIssuer {
            tenant_id: check_tenant_id(tenant_id)?,
        })
    }

    /// The PRF input: fields separated by 0x00, ending with the purpose and version.
    fn prf_input(&self, domain: &Domain) -> Vec<u8> {
        let org;
        let fields: Vec<&str> = match self {
            Self::Organisation {
                tenant_id,
                org_id,
                scope,
            } => {
                org = org_id.hyphenated().to_string();
                vec![
                    tenant_id.as_str(),
                    org.as_str(),
                    domain.as_str(),
                    scope.as_str(),
                    ORG_DOMAIN_PURPOSE,
                    TOKEN_VERSION,
                ]
            }
            Self::TenantIssuer { tenant_id } => vec![
                tenant_id.as_str(),
                domain.as_str(),
                TENANT_ISSUER_PURPOSE,
                TOKEN_VERSION,
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

fn mac(key: &[u8], subject: &ClaimSubject, domain: &Domain) -> HmacSha256 {
    // HMAC accepts keys of any length; `new_from_slice` cannot fail for it.
    #[allow(clippy::expect_used)]
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC takes any key length");
    mac.update(&subject.prf_input(domain));
    mac
}

/// The token for `subject` and `domain` under `key` (43 characters of base64url).
///
/// Exposed for test vectors and for components that hold their own key; services use [`EnrollKey`].
#[must_use]
pub fn derive_token(key: &[u8], subject: &ClaimSubject, domain: &Domain) -> String {
    URL_SAFE_NO_PAD.encode(mac(key, subject, domain).finalize().into_bytes())
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

struct EpochKey {
    epoch: u32,
    key: Vec<u8>,
}

/// The enrolment key `K_enroll`: the current epoch and, during rotation, the previous one.
///
/// Born in the secret backend and mounted as a file (ADR-006); never in git. The file is JSON:
///
/// ```json
/// {"current": {"epoch": 2, "key": "<base64url, at least 32 bytes>"},
///  "previous": {"epoch": 1, "key": "<base64url>"}}
/// ```
///
/// `previous` is optional. To rotate: generate a new key as `current` with the next epoch and move the old
/// current to `previous`; records under the previous epoch keep verifying until it is dropped.
/// `Debug` never shows key material, and load errors never quote the file.
pub struct EnrollKey {
    current: EpochKey,
    previous: Option<EpochKey>,
}

impl fmt::Debug for EnrollKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EnrollKey")
            .field("current_epoch", &self.current.epoch)
            .field("previous_epoch", &self.previous.as_ref().map(|k| k.epoch))
            .field("keys", &"<redacted>")
            .finish()
    }
}

/// Why an enrolment key could not be loaded. Never contains key material or file contents.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EnrollKeyError {
    /// The file could not be read (the I/O error kind only).
    #[error("enrolment key file unreadable: {0}")]
    Unreadable(String),
    /// The file is not the expected JSON shape.
    #[error("enrolment key file malformed: {0}")]
    Malformed(&'static str),
    /// A key shorter than 256 bits.
    #[error("enrolment key shorter than 32 bytes")]
    TooShort,
    /// `current` and `previous` must have different epochs, and the same key must not serve two epochs.
    #[error("enrolment key epochs must differ")]
    EpochClash,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct EpochKeyFile {
    epoch: u32,
    key: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct EnrollKeyFile {
    current: EpochKeyFile,
    previous: Option<EpochKeyFile>,
}

fn decode_key(file: &EpochKeyFile) -> Result<EpochKey, EnrollKeyError> {
    let key = URL_SAFE_NO_PAD
        .decode(file.key.trim_end_matches('='))
        .map_err(|_| EnrollKeyError::Malformed("key is not base64url"))?;
    if key.len() < MIN_KEY_BYTES {
        return Err(EnrollKeyError::TooShort);
    }
    Ok(EpochKey {
        epoch: file.epoch,
        key,
    })
}

impl EnrollKey {
    /// Build from raw keys (tests and tools).
    ///
    /// # Errors
    ///
    /// [`EnrollKeyError::TooShort`] or [`EnrollKeyError::EpochClash`].
    pub fn new(
        current: (u32, Vec<u8>),
        previous: Option<(u32, Vec<u8>)>,
    ) -> Result<Self, EnrollKeyError> {
        let make = |(epoch, key): (u32, Vec<u8>)| {
            if key.len() < MIN_KEY_BYTES {
                Err(EnrollKeyError::TooShort)
            } else {
                Ok(EpochKey { epoch, key })
            }
        };
        let current = make(current)?;
        let previous = previous.map(make).transpose()?;
        Self::checked(current, previous)
    }

    fn checked(current: EpochKey, previous: Option<EpochKey>) -> Result<Self, EnrollKeyError> {
        if let Some(prev) = &previous {
            if prev.epoch == current.epoch || prev.key == current.key {
                return Err(EnrollKeyError::EpochClash);
            }
        }
        Ok(Self { current, previous })
    }

    /// Parse the JSON key file contents.
    ///
    /// # Errors
    ///
    /// See [`EnrollKeyError`]; messages never quote the input.
    pub fn from_json(bytes: &[u8]) -> Result<Self, EnrollKeyError> {
        let file: EnrollKeyFile = serde_json::from_slice(bytes)
            .map_err(|_| EnrollKeyError::Malformed("expected {current:{epoch,key}, previous?}"))?;
        let current = decode_key(&file.current)?;
        let previous = file.previous.as_ref().map(decode_key).transpose()?;
        Self::checked(current, previous)
    }

    /// Read and parse the key file (a mounted secret).
    ///
    /// # Errors
    ///
    /// See [`EnrollKeyError`].
    pub fn from_file(path: &Path) -> Result<Self, EnrollKeyError> {
        let meta = std::fs::metadata(path)
            .map_err(|e| EnrollKeyError::Unreadable(e.kind().to_string()))?;
        if meta.len() > MAX_KEY_FILE {
            return Err(EnrollKeyError::Malformed("file too large"));
        }
        let bytes =
            std::fs::read(path).map_err(|e| EnrollKeyError::Unreadable(e.kind().to_string()))?;
        Self::from_json(&bytes)
    }

    /// Epoch new records are issued under.
    #[must_use]
    pub fn current_epoch(&self) -> u32 {
        self.current.epoch
    }

    /// The record to publish for `subject` and `domain`, under the current epoch.
    #[must_use]
    pub fn expected_record(&self, subject: &ClaimSubject, domain: &Domain) -> TxtRecord {
        TxtRecord {
            name: challenge_name(domain),
            value: format!(
                "{RECORD_PREFIX}{TOKEN_VERSION};t={}",
                derive_token(&self.current.key, subject, domain)
            ),
            epoch: self.current.epoch,
        }
    }

    /// Check the TXT values found at the challenge name. Returns the epoch whose key matched, or `None`.
    ///
    /// Each value is one TXT record (its character-strings concatenated). Non-Sesame and other-version
    /// strings are skipped. Comparison is constant-time (`hmac::Mac::verify_slice`) against the current and,
    /// if loaded, the previous epoch key.
    pub fn verify<I, S>(
        &self,
        subject: &ClaimSubject,
        domain: &Domain,
        txt_values: I,
    ) -> Option<u32>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let keys: Vec<&EpochKey> = std::iter::once(&self.current)
            .chain(self.previous.as_ref())
            .collect();
        let mut matched = None;
        for value in txt_values {
            let Some(token) = parse_record(value.as_ref()) else {
                continue;
            };
            let Ok(presented) = URL_SAFE_NO_PAD.decode(token) else {
                continue;
            };
            if presented.len() != TOKEN_BYTES {
                continue;
            }
            for key in &keys {
                // Every candidate is checked (no early exit), so timing does not reveal which matched.
                if mac(&key.key, subject, domain)
                    .verify_slice(&presented)
                    .is_ok()
                    && matched.is_none()
                {
                    matched = Some(key.epoch);
                }
            }
        }
        matched
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain_verify::normalize_domain;

    const ORG: Uuid = Uuid::from_u128(0xb200_0002_0002_4000_8000_0000_0000_0002);

    fn key(byte: u8) -> Vec<u8> {
        vec![byte; 32]
    }

    fn domain(d: &str) -> Domain {
        normalize_domain(d).expect("valid")
    }

    fn org(scope: Scope) -> ClaimSubject {
        ClaimSubject::organisation("acme", ORG, scope).expect("valid")
    }

    #[test]
    fn record_shape() {
        let k = EnrollKey::new((7, key(1)), None).expect("key");
        let rec = k.expected_record(&org(Scope::Exact), &domain("BigBank.com"));
        assert_eq!(rec.name, "_sesame-challenge.bigbank.com");
        assert!(rec.value.starts_with("sesame-domain-verify=v1;t="));
        assert_eq!(rec.value.len(), RECORD_PREFIX.len() + 5 + TOKEN_LEN);
        assert_eq!(rec.epoch, 7);
        assert!(parse_record(&rec.value).is_some());
    }

    #[test]
    fn verify_accepts_current_and_previous_only() {
        let d = domain("bigbank.com");
        let s = org(Scope::Exact);
        let old = EnrollKey::new((1, key(1)), None).expect("key");
        let rotated = EnrollKey::new((2, key(2)), Some((1, key(1)))).expect("key");
        let retired = EnrollKey::new((3, key(3)), Some((2, key(2)))).expect("key");
        let published = old.expected_record(&s, &d).value;
        assert_eq!(rotated.verify(&s, &d, [&published]), Some(1));
        assert_eq!(retired.verify(&s, &d, [&published]), None);
        let fresh = rotated.expected_record(&s, &d).value;
        assert_eq!(rotated.verify(&s, &d, [&fresh]), Some(2));
    }

    #[test]
    fn verify_finds_its_record_among_others() {
        let d = domain("bigbank.com");
        let s = org(Scope::Exact);
        let k = EnrollKey::new((1, key(9)), None).expect("key");
        let other = ClaimSubject::organisation("other-tenant", ORG, Scope::Exact).expect("valid");
        let values = vec![
            "v=spf1 -all".to_string(),
            k.expected_record(&other, &d).value,
            "sesame-domain-verify=v2;t=whatever".to_string(),
            k.expected_record(&s, &d).value,
        ];
        assert_eq!(k.verify(&s, &d, &values), Some(1));
        assert_eq!(k.verify(&s, &d, &values[..3]), None);
    }

    #[test]
    fn malformed_records_never_match() {
        let d = domain("bigbank.com");
        let s = org(Scope::Exact);
        let k = EnrollKey::new((1, key(5)), None).expect("key");
        let good = k.expected_record(&s, &d).value;
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
            assert_eq!(k.verify(&s, &d, [&bad]), None, "{bad:?}");
        }
    }

    #[test]
    fn binding_to_subject_domain_and_scope() {
        let k = EnrollKey::new((1, key(4)), None).expect("key");
        let d = domain("bigbank.com");
        let published = k.expected_record(&org(Scope::Exact), &d).value;
        assert_eq!(k.verify(&org(Scope::Subdomains), &d, [&published]), None);
        assert_eq!(
            k.verify(&org(Scope::Exact), &domain("eu.bigbank.com"), [&published]),
            None
        );
        let issuer = ClaimSubject::tenant_issuer("acme").expect("valid");
        assert_eq!(k.verify(&issuer, &d, [&published]), None);
        let other_org =
            ClaimSubject::organisation("acme", Uuid::from_u128(1), Scope::Exact).expect("valid");
        assert_eq!(k.verify(&other_org, &d, [&published]), None);
    }

    #[test]
    fn subjects_validate_tenant_id() {
        assert!(ClaimSubject::tenant_issuer("").is_err());
        assert!(ClaimSubject::tenant_issuer("a\0b").is_err());
        assert!(ClaimSubject::tenant_issuer(&"t".repeat(65)).is_err());
        assert!(ClaimSubject::tenant_issuer(&"t".repeat(64)).is_ok());
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

    #[test]
    fn key_file_parsing_and_redaction() {
        let k1 = URL_SAFE_NO_PAD.encode([0x11u8; 32]);
        let k2 = URL_SAFE_NO_PAD.encode([0x22u8; 32]);
        let json = format!(
            r#"{{"current":{{"epoch":2,"key":"{k2}"}},"previous":{{"epoch":1,"key":"{k1}"}}}}"#
        );
        let k = EnrollKey::from_json(json.as_bytes()).expect("parses");
        assert_eq!(k.current_epoch(), 2);
        let debug = format!("{k:?}");
        assert!(!debug.contains(&k1) && !debug.contains(&k2), "{debug}");
        assert!(debug.contains("redacted"));

        let short = URL_SAFE_NO_PAD.encode([0x33u8; 16]);
        let json = format!(r#"{{"current":{{"epoch":1,"key":"{short}"}}}}"#);
        assert_eq!(
            EnrollKey::from_json(json.as_bytes()).err(),
            Some(EnrollKeyError::TooShort)
        );

        let clash = format!(
            r#"{{"current":{{"epoch":1,"key":"{k1}"}},"previous":{{"epoch":1,"key":"{k2}"}}}}"#
        );
        assert_eq!(
            EnrollKey::from_json(clash.as_bytes()).err(),
            Some(EnrollKeyError::EpochClash)
        );

        // Errors never quote the input, even when it holds key material.
        let leaky = format!(r#"{{"current":{{"epoch":"x","key":"{k1}"}}}}"#);
        let err = EnrollKey::from_json(leaky.as_bytes()).expect_err("malformed");
        assert!(!err.to_string().contains(&k1));
        let unknown = format!(r#"{{"current":{{"epoch":1,"key":"{k1}","note":"{k2}"}}}}"#);
        let err = EnrollKey::from_json(unknown.as_bytes()).expect_err("unknown field");
        assert!(!err.to_string().contains(&k2));
    }
}
