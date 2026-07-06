//! Data structures, error types, metrics, and standalone functions for the fallback cache.

use std::time::Duration;

use prometheus::{Gauge, HistogramVec, IntCounter, IntCounterVec, Registry};
use serde::{Deserialize, Serialize};

// ===========================================================================
// AuthzDecision
// ===========================================================================

/// Authorization decision result from authz-core or JWT common path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuthzDecision {
    /// Allowed — the request is authorized.
    Allowed { reason: String },
    /// Denied — the request is not authorized.
    Denied { reason: String },
}

impl AuthzDecision {
    #[must_use]
    pub fn is_allowed(&self) -> bool {
        matches!(self, AuthzDecision::Allowed { .. })
    }

    /// Serialize to JSON string for Redis storage.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            serde_json::to_string(&Self::Allowed {
                reason: "serialization_error".to_string(),
            })
            .unwrap()
        })
    }

    /// Deserialize from JSON string from Redis.
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

// ===========================================================================
// AuthzCheckRequest
// ===========================================================================

/// Authorization check request — what gets sent to authz-core /authorize.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthzCheckRequest {
    pub tenant_id: String,
    pub sub: String,
    pub org_id: String,
    pub action: String,
    pub resource_id: String,
}

impl AuthzCheckRequest {
    /// Generate a deterministic cache key for this request.
    ///
    /// Cache key format: `authz_fallback:{blake3_hash}`
    #[must_use]
    pub fn cache_key(&self) -> String {
        let key_data = format!(
            "{}:{}:{}:{}:{}",
            self.tenant_id, self.sub, self.org_id, self.action, self.resource_id
        );
        let hash = blake3::hash(key_data.as_bytes());
        format!("authz_fallback:{hash}")
    }

    /// Generate a blake3 hex digest of the key data.
    #[must_use]
    pub fn cache_key_hash(&self) -> String {
        let key_data = format!(
            "{}:{}:{}:{}:{}",
            self.tenant_id, self.sub, self.org_id, self.action, self.resource_id
        );
        blake3::hash(key_data.as_bytes()).to_hex().to_string()
    }
}

// ===========================================================================
// RouteFallbackPolicy
// ===========================================================================

/// Route-specific policy for fallback caching.
/// Loaded from `RoutePolicyStore` at startup.
#[derive(Debug, Clone)]
pub struct RouteFallbackPolicy {
    /// Route path pattern (e.g., "/admin/users/me/preferences")
    pub route: String,
    /// HTTP method (e.g., "PUT", "POST", "PATCH")
    pub method: String,
    /// Cache TTL in seconds (5-30)
    pub cache_ttl_seconds: u64,
    /// Whether this route requires fallback at all
    pub requires_fallback: bool,
}

impl RouteFallbackPolicy {
    pub fn new(
        route: impl Into<String>,
        method: impl Into<String>,
        cache_ttl_seconds: u64,
        requires_fallback: bool,
    ) -> Self {
        Self {
            route: route.into(),
            method: method.into(),
            cache_ttl_seconds,
            requires_fallback,
        }
    }
}

// ===========================================================================
// FallbackMetrics
// ===========================================================================

/// Metrics for fallback cache monitoring.
pub struct FallbackMetrics {
    registry: Registry,
    pub total: IntCounterVec,
    pub cache_hit_total: IntCounter,
    pub cache_miss_total: IntCounter,
    pub cache_size: Gauge,
    pub cache_hit_ratio: Gauge,
    pub latency: HistogramVec,
}

impl FallbackMetrics {
    /// Create and register all fallback cache metrics with the given registry.
    #[must_use]
    pub fn new(registry: &Registry) -> Self {
        let total = IntCounterVec::new(
            prometheus::Opts::new(
                "authz_fallback_total",
                "Total fallback requests per route and result",
            ),
            &["route", "result"],
        )
        .unwrap();
        registry.register(Box::new(total.clone())).unwrap();

        let cache_hit_total = IntCounter::new(
            "authz_fallback_cache_hit_total",
            "Total cache hits (cached results returned without calling authz-core)",
        )
        .unwrap();
        registry
            .register(Box::new(cache_hit_total.clone()))
            .unwrap();

        let cache_miss_total = IntCounter::new(
            "authz_fallback_cache_miss_total",
            "Total cache misses (authz-core was called)",
        )
        .unwrap();
        registry
            .register(Box::new(cache_miss_total.clone()))
            .unwrap();

        let cache_size = Gauge::new(
            "authz_fallback_cache_size",
            "Current number of entries in the fallback cache",
        )
        .unwrap();
        registry.register(Box::new(cache_size.clone())).unwrap();

        let cache_hit_ratio = Gauge::new(
            "authz_fallback_cache_hit_ratio",
            "Cache hit ratio (cache_hits / (cache_hits + cache_misses))",
        )
        .unwrap();
        registry
            .register(Box::new(cache_hit_ratio.clone()))
            .unwrap();

        let latency = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "authz_fallback_latency_ms",
                "Latency of fallback calls to authz-core in milliseconds",
            ),
            &["route"],
        )
        .unwrap();
        registry.register(Box::new(latency.clone())).unwrap();

        Self {
            registry: registry.clone(),
            total,
            cache_hit_total,
            cache_miss_total,
            cache_size,
            cache_hit_ratio,
            latency,
        }
    }

    /// Increment the total counter for a specific route and result type.
    pub fn inc_total(&self, route: &str, result: &str) {
        self.total.with_label_values(&[route, result]).inc();
    }

    /// Increment cache hit counter and update hit ratio.
    pub fn inc_cache_hit(&self) {
        self.cache_hit_total.inc();
        self.update_hit_ratio();
    }

    /// Increment cache miss counter and update hit ratio.
    pub fn inc_cache_miss(&self) {
        self.cache_miss_total.inc();
        self.update_hit_ratio();
    }

    /// Recompute the hit ratio gauge from the hit/miss counters.
    fn update_hit_ratio(&self) {
        let hits = self.cache_hit_total.get();
        let misses = self.cache_miss_total.get();
        let total = hits + misses;
        if total > 0 {
            self.cache_hit_ratio.set(hits as f64 / total as f64);
        } else {
            self.cache_hit_ratio.set(0.0);
        }
    }

    /// Update cache size gauge.
    pub fn set_cache_size(&self, size: u64) {
        self.cache_size.set(size as f64);
    }

    /// Record the latency of a fallback call.
    pub fn record_latency(&self, route: &str, latency_ms: f64) {
        self.latency.with_label_values(&[route]).observe(latency_ms);
    }
}

// ===========================================================================
// SingleFlightResult
// ===========================================================================

/// Result of a single-flight authorization check.
#[derive(Debug)]
pub struct SingleFlightResult {
    pub decision: AuthzDecision,
    pub is_cache: bool,
}

// ===========================================================================
// Per-route TTL configuration
// ===========================================================================

/// Per-route TTL configuration: route path -> TTL in seconds.
pub type RouteTtlConfig = std::collections::HashMap<String, u64>;

/// Default TTL values per route category.
///
/// Per Story 7.2: write-type actions get 30s, data integrity routes get 15s.
/// Unknown routes default to 15 seconds.
#[must_use]
pub fn default_ttl() -> RouteTtlConfig {
    let mut config = RouteTtlConfig::new();
    // preferences PUT — low-risk write, 30s acceptable
    config.insert("/admin/users/me/preferences".to_string(), 30);
    // email upsert — data integrity needs more freshness
    config.insert("/admin/users/me/email".to_string(), 15);
    // user update — ownership from JWT
    config.insert("/admin/users/me".to_string(), 30);
    // admin query — tenant-scoped
    config.insert("/admin/users/query".to_string(), 15);
    config
}

/// Sanitize input strings for cache key generation.
///
/// SECURITY (HACK-722): Strip control characters (ASCII < 0x20) and
/// truncate to prevent oversized cache keys.
#[must_use]
pub fn sanitize_key_input(s: &str, max_len: usize) -> String {
    // Strip control characters (including newlines/tabs, which could inject
    // Redis protocol frames) but preserve printable unicode (HACK-722).
    s.chars()
        .filter(|c| !c.is_control())
        .take(max_len)
        .collect()
}

// ===========================================================================
// jwt_claims_cover_decision
// ===========================================================================

/// Determine if JWT claims are sufficient to make this authorization decision.
///
/// If the user has the required roles in `claims.sx.roles` or the required
/// permissions in `claims.sx.permissions`, return `true` (short-circuit, no cache needed).
#[must_use]
pub fn jwt_claims_cover_decision(
    claims_roles: &[String],
    claims_permissions: &[String],
    roles_required: &[&str],
    permissions_required: &[&str],
) -> bool {
    // No requirements at all — trivially covered, no online check needed.
    if roles_required.is_empty() && permissions_required.is_empty() {
        return true;
    }

    // Check if any required role is present
    for required_role in roles_required {
        if claims_roles.iter().any(|r| r.as_str() == *required_role) {
            return true;
        }
    }

    // Check if any required permission is present
    for required_perm in permissions_required {
        if claims_permissions
            .iter()
            .any(|p| p.as_str() == *required_perm)
        {
            return true;
        }
    }

    false
}

// ===========================================================================
// FallbackCacheError
// ===========================================================================

/// Errors specific to the fallback cache.
#[derive(Debug, Clone, PartialEq)]
pub enum FallbackCacheError {
    /// Redis connection error
    RedisError(String),
    /// Authz-core returned an error (not cached per HACK-721)
    AuthzCoreError { status: u16, reason: String },
    /// JSON serialization error
    JsonError(String),
    /// Single-flight timed out waiting for in-flight request
    SingleFlightTimeout(Duration),
}

impl std::fmt::Display for FallbackCacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FallbackCacheError::RedisError(msg) => write!(f, "Redis error: {msg}"),
            FallbackCacheError::AuthzCoreError { status, reason } => {
                write!(f, "Authz-core returned {status} ({reason})")
            }
            FallbackCacheError::JsonError(msg) => write!(f, "JSON error: {msg}"),
            FallbackCacheError::SingleFlightTimeout(duration) => {
                write!(f, "Single-flight timed out after {duration:?}")
            }
        }
    }
}

impl std::error::Error for FallbackCacheError {}

impl From<redis::RedisError> for FallbackCacheError {
    fn from(e: redis::RedisError) -> Self {
        FallbackCacheError::RedisError(e.to_string())
    }
}

impl From<serde_json::Error> for FallbackCacheError {
    fn from(e: serde_json::Error) -> Self {
        FallbackCacheError::JsonError(e.to_string())
    }
}
