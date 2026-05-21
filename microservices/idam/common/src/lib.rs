//! Shared JWT claim types, validation, builder, middleware, and JWKS cache for Sesame-IDAM microservices.
//!
//! This crate provides:
//! - `jwt` module — JWT claim structures (AccessClaims, SesameAuthzClaims, ActorClaim)
//! - `middleware` module — JWT Common-Path Authorization Middleware for BRRTRouter
//! - `jwks_cache` module — JWKS cache with background refresh, stale tolerance, and security protections
//!
//! The middleware enables fast-path authorization for `jwt-only` routes without
//! calling authz-core, by evaluating policy locally from JWT claims.

pub mod jwt;
pub mod middleware;
pub mod jwks_cache;

pub use jwt::{
    AccessClaims, ActorClaim, JwtError, JwtValidationError, SesameAuthzClaims,
    ALLOWED_ISSUERS, EXPECTED_AUDIENCE,
};
