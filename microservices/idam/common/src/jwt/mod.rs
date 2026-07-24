//! JWT claim structures and utilities for Sesame-IDAM.
//!
//! Modular layout:
//! - [`types`] — Core structs/enums (`AccessClaims`, `SesameAuthzClaims`, `ActorClaim`, etc.)
//! - [`builders`] — Builder patterns for `AccessClaims` and `SesameAuthzClaims`
//! - [`helpers`] — Entitlements refs/hashes, size enforcement, constants
//! - [`tests`] — Unit tests (46 functions)
//!
//! ## PII Removal (Story 2.3)
//!
//! PII fields are REMOVED from JWT access tokens. Consumers fetch PII from
//! GET /api/v1/identity/users/me.
//!
//! ## Entitlements Reference Pattern
//!
//! Full permissions array replaced with:
//! - `entitlements_ref` — deterministic UUID-based Redis lookup key
//! - `entitlements_hash` — SHA-256 hash of canonical JSON for cache verification

pub mod builders;
pub mod helpers;
pub mod keyset;
pub mod signer;
pub mod types;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Re-exports for consumers of sesame_common::jwt
// ---------------------------------------------------------------------------

pub use builders::{AccessClaimsBuilder, SesameAuthzClaimsBuilder};
pub use helpers::{
    compute_entitlements_hash, generate_entitlements_ref, measure_jwt_token_size,
    truncate_authz_claims_permissions, truncate_permissions, validate_entitlements_ref,
    verify_entitlements_hash, ALLOWED_ISSUERS, EXPECTED_AUDIENCE, MAX_ENTITLEMENTS_REF_LENGTH,
    MAX_PERMISSIONS_PER_ROLE, MAX_TOKEN_SIZE_BYTES, TOKEN_SIZE_WARNING_BYTES,
};
pub use keyset::{
    configured_keyset_file, load_keyset_file, parse_keyset, rfc7638_okp_thumbprint, signing_key,
    KeysetError, LoadedKey, SigningKeyset, KEYSET_FILE_ENV, KEY_SOURCE_ENV,
};
pub use signer::{Ed25519Signer, SignerError, SIGNING_KEY_ENV, SIGNING_KID_ENV};
pub use types::{
    AccessClaims, ActorClaim, EntitlementsSnapshot, JwtError, JwtValidationError, SesameAuthzClaims,
};
