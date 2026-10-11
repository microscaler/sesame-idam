//! Sesame `saml-verifier` (ADR-014, Epic 17 story 17.6).
//!
//! Verifies SAML responses for identity-login-service with samael and libxmlsec1, in a process that holds no
//! secrets and reaches nothing. Sesame's own checks wrap samael's:
//!
//! 1. [`precheck`]: size, strict base64, UTF-8, no DOCTYPE, comments, processing instructions, CDATA or encrypted
//!    content, exactly one assertion, a constrained signature shape;
//! 2. samael with libxmlsec1: signatures against the connection's pinned certificates only (at least one required),
//!    an algorithm allow-list, reduction to verified content only, issuer, recipient, `InResponseTo`, time windows;
//! 3. [`verify`] post-checks: the verified assertion is the document's only assertion, status Success,
//!    destination, audience, an `AuthnStatement`, a non-empty `NameID`.
//!
//! Every refusal is a fixed [`Refusal`] code; nothing from the input is echoed or logged.

pub mod metadata;
pub mod precheck;
pub mod verify;

pub use verify::{verify, VerifiedAssertion, VerifyRequest};

/// Why a response was refused. The code is all a caller (and the log) ever sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    /// Request body or response over the size limit.
    #[error("too_large")]
    TooLarge,
    /// Not strict base64.
    #[error("bad_base64")]
    BadBase64,
    /// Not UTF-8.
    #[error("not_utf8")]
    NotUtf8,
    /// Not well-formed XML, or nested too deeply.
    #[error("malformed")]
    Malformed,
    /// The root is not a single `samlp:Response`.
    #[error("not_a_response")]
    NotAResponse,
    /// A DOCTYPE (entities) is present.
    #[error("doctype")]
    Doctype,
    /// An XML comment is present.
    #[error("comment")]
    Comment,
    /// A processing instruction is present.
    #[error("processing_instruction")]
    ProcessingInstruction,
    /// A CDATA section is present.
    #[error("cdata")]
    Cdata,
    /// Encrypted assertion or NameID (not supported, ADR-014).
    #[error("encrypted")]
    Encrypted,
    /// Not exactly one assertion.
    #[error("assertion_count")]
    AssertionCount,
    /// Signature structure outside what SAML needs (too many signatures, `Object`, unknown transforms).
    #[error("signature_shape")]
    SignatureShape,
    /// No pinned certificate supplied, or one that is not base64 DER.
    #[error("no_certificate")]
    NoCertificate,
    /// The signature did not verify against the pinned certificates.
    #[error("signature")]
    Signature,
    /// Issuer is not the connection's IdP.
    #[error("issuer")]
    Issuer,
    /// `InResponseTo` does not name the pending request.
    #[error("in_response_to")]
    InResponseTo,
    /// Destination or recipient is not this connection's ACS.
    #[error("destination")]
    Destination,
    /// Audience is not this connection's SP entity ID.
    #[error("audience")]
    Audience,
    /// Outside the validity window.
    #[error("time")]
    Time,
    /// Status is not Success.
    #[error("status")]
    Status,
    /// No `AuthnStatement`.
    #[error("no_authn_statement")]
    NoAuthnStatement,
    /// Empty or missing `NameID`.
    #[error("name_id")]
    NameId,
    /// The verified assertion is not the document's assertion.
    #[error("assertion_id_mismatch")]
    AssertionIdMismatch,
    /// Not usable IdP metadata (no single IdP, no HTTPS HTTP-Redirect sign-on service, unparsable).
    #[error("metadata")]
    Metadata,
    /// A request field is missing or invalid.
    #[error("bad_request")]
    BadRequest,
    /// Anything else samael refused.
    #[error("invalid")]
    Invalid,
}

impl Refusal {
    /// The stable code.
    #[must_use]
    pub fn code(self) -> String {
        self.to_string()
    }
}
