//! Online fallback result cache for hybrid authorization (Story 7.2).
//!
//! Redis-based caching for authorization decisions on `jwt-with-fallback` routes.
//!
//! Key features:
//! - Cache backend: **Redis** (shared across services)
//! - Cache key: `authz_fallback:{blake3_hash}` — hash of subject + org + action + `resource_id`
//! - Per-route TTL (5-30 seconds) from `RoutePolicyStore`
//! - Single-flight pattern: only ONE request hits authz-core per cache key
//! - Metrics: `authz_fallback_cache_hit_ratio`, `authz_fallback_cache_size`, `authz_fallback_cache_miss_total`
//! - Write-type actions use short TTL (5s) or are excluded (HACK-721)
//! - Redis unavailable -> fall through to authz-core directly (fail-open)
//!
//! Dependencies: blake3, redis, prometheus, serde, `serde_json`

pub mod cache;
pub mod redis;
pub mod types;

// Re-export types and helpers from sub-modules so the public API is unchanged.
pub use types::{
    default_ttl, jwt_claims_cover_decision, sanitize_key_input, AuthzCheckRequest, AuthzDecision,
    FallbackCacheError, FallbackMetrics, RouteFallbackPolicy, RouteTtlConfig, SingleFlightResult,
};

// Re-export the cache struct so callers use the canonical import path.
pub use cache::FallbackCache;

#[cfg(test)]
mod tests;
