//! Shared JWT claim types, validation, builder, and middleware for Sesame-IDAM microservices.
//!
//! This crate provides:
//! - `jwt` module — JWT claim structures (AccessClaims, SesameAuthzClaims, ActorClaim)
//! - `middleware` module — JWT Common-Path Authorization Middleware for BRRTRouter
//!
//! The middleware enables fast-path authorization for `jwt-only` routes without
//! calling authz-core, by evaluating policy locally from JWT claims.

pub mod jwt;
pub mod middleware;

pub use jwt::{
    AccessClaims, ActorClaim, EntitlementsSnapshot, JwtError, JwtValidationError,
    SesameAuthzClaims, generate_entitlements_ref, compute_entitlements_hash,
    verify_entitlements_hash,
};
