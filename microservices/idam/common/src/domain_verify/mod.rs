//! Domain claim primitives (Epic 17, story 17.1).
//!
//! Everything that decides whether a DNS TXT record proves control of a domain (no DNS I/O here):
//!
//! - [`normalize_domain`]: user input to a canonical [`Domain`] (lower-case IDNA A-label, no trailing dot).
//! - [`check_eligible`]: public suffixes, unlisted top-level domains, consumer mail providers and the
//!   deployment's own domains can never be claimed.
//! - [`ClaimSubject`], [`challenge_name`], [`expected_record`], [`verify_record`]: the record an owner publishes,
//!   `_sesame-challenge.<domain> TXT "sesame-domain-verify=v1;t=<token>"`, and its constant-time check.
//! - [`EnrollKeyring`]: where `K_enroll` lives. [`GcpKmsKeyring`] keeps it in Cloud KMS at HSM protection level
//!   (production); [`LocalKeyring`] reads a key file (development and tests).
//!
//! Token construction (`DESIGN-org-domain-verification.md` §3, `DESIGN-tenant-domain-verification.md` §2):
//!
//! ```text
//! organisation:  HMAC-SHA256(K_epoch, tenant_id 0 org_id 0 claim_id 0 fqdn 0 scope 0 "sesame-org-domain" 0 "v1")
//! tenant issuer: HMAC-SHA256(K_epoch, tenant_id 0 claim_id 0 fqdn 0 "sesame-domain-verify" 0 "v1")
//! token = base64url, no padding (43 characters)
//! ```
//!
//! [`dns`] encodes and decodes the challenge lookup for `dns-probe`; [`quorum`] decides the verdict from its
//! answers (story 17.2). [`lifecycle`] decides every state change of a claim (story 17.3).

pub mod dns;
pub mod doh;
mod eligibility;
mod gcp_kms;
mod keyring;
pub mod lifecycle;
mod normalize;
mod quorum;
mod token;

pub use eligibility::{check_eligible, EligibilityPolicy, DEFAULT_CONSUMER_PROVIDERS};
pub use gcp_kms::{GcpKmsConfig, GcpKmsKeyring};
pub use keyring::{EnrollKeyring, KeyringConfig, KeyringError, LocalKeyring, MAC_LEN};
pub use lifecycle::{
    apply as apply_claim_event, Claim, ClaimEvent, ClaimState, LifecycleError, LifecycleEvent,
    Withdrawer,
};
pub use normalize::{normalize_domain, Domain};
pub use quorum::{quorum, ProviderAnswer, ProviderOutcome, Verdict, QUORUM};
pub use token::{
    challenge_name, derive_token, expected_record, parse_record, verify_record, ClaimSubject,
    Scope, TxtRecord, CHALLENGE_LABEL, ORG_DOMAIN_PURPOSE, RECORD_PREFIX, TENANT_ISSUER_PURPOSE,
    TOKEN_VERSION,
};

/// Why a domain cannot be claimed. Messages never echo the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    /// Not a syntactically valid DNS host name after normalisation; the reason is fixed text.
    #[error("invalid domain: {0}")]
    InvalidDomain(&'static str),
    /// A public suffix (`com`, `co.uk`, `github.io`) or a name above a registrable domain.
    #[error("domain is a public suffix")]
    PublicSuffix,
    /// The top-level domain is not on the Public Suffix List compiled into this build.
    #[error("top-level domain is not listed")]
    UnknownSuffix,
    /// A consumer mail provider (`gmail.com`, `outlook.com`, …) or one of its subdomains.
    #[error("domain belongs to a consumer mail provider")]
    ConsumerProvider,
    /// One of the deployment's own domains, or a subdomain of one.
    #[error("domain is reserved")]
    ReservedDomain,
    /// A tenant or organisation identifier unusable in a claim (empty, too long, or containing NUL).
    #[error("invalid claim subject: {0}")]
    InvalidSubject(&'static str),
}

impl DomainError {
    /// Stable machine-readable code (logs, metrics, API mapping).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidDomain(_) => "invalid_domain",
            Self::PublicSuffix => "public_suffix",
            Self::UnknownSuffix => "unknown_suffix",
            Self::ConsumerProvider => "consumer_provider",
            Self::ReservedDomain => "reserved_domain",
            Self::InvalidSubject(_) => "invalid_subject",
        }
    }
}
