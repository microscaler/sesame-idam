//! Shared JWT claim types, validation, builder, middleware, and JWKS cache for Sesame-IDAM microservices.
//!
//! This crate provides:
//! - `jwt` module — JWT claim structures (AccessClaims, SesameAuthzClaims, ActorClaim)
//! - `middleware` module — JWT Common-Path Authorization Middleware for BRRTRouter
//! - `jwks_cache` module — JWKS cache with background refresh, stale tolerance, and security protections
//! - `fallback_cache` module — Selective online fallback with Redis caching (Story 4.3)
//!
//! The middleware enables fast-path authorization for `jwt-only` routes without
//! calling authz-core, by evaluating policy locally from JWT claims.

pub mod dpop;
#[cfg(feature = "fallback_cache")]
pub mod fallback_cache;
#[cfg(feature = "jwks_cache")]
pub mod jwks_cache;
pub mod jwt;
pub mod middleware;

pub use jwt::{
    AccessClaims, AccessClaimsBuilder, ActorClaim, JwtError, JwtValidationError, SesameAuthzClaims,
    SesameAuthzClaimsBuilder, ALLOWED_ISSUERS, EXPECTED_AUDIENCE,
};
