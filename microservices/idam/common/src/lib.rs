//! Shared JWT claim types, validation, builder, middleware, JWKS cache, and DPoP for Sesame-IDAM microservices.
//!
//! This crate provides:
//! - `jwt` module — JWT claim structures (AccessClaims, SesameAuthzClaims, ActorClaim)
//! - `middleware` module — JWT Common-Path Authorization Middleware for BRRTRouter
//! - `jwks_cache` module — JWKS cache with background refresh, stale tolerance, and security protections
//! - `dpop` module — DPoP (RFC 9449) proof validation, key generation, and JTI replay detection
//!
//! The middleware enables fast-path authorization for `jwt-only` routes without
//! calling authz-core, by evaluating policy locally from JWT claims.

pub mod dpop;
pub mod jwt;
#[cfg(feature = "jwks_cache")]
pub mod jwks_cache;
pub mod middleware;

pub use jwt::{
    AccessClaims, ActorClaim, JwtError, JwtValidationError, SesameAuthzClaims,
    ALLOWED_ISSUERS, EXPECTED_AUDIENCE,
};
pub use dpop::{
    DpopConfirmation, DpopError, DpopJtiStore, DpopJwk, DpopProof, InMemoryJtiStore,
    compute_jkt, create_dpop_proof_jwt, generate_dpop_key_pair, generate_dpop_key_pair_p256,
    verify_dpop_proof,
};
