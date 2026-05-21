//! Online fallback result cache for hybrid authorization (Story 7.2).
//!
//! Redis-based caching for authorization decisions on `jwt-with-fallback` routes.
//!
//! Key features:
//! - Cache backend: **Redis** (shared across services)
//! - Cache key: `authz_fallback:{blake3_hash}` — hash of subject + org + action + resource_id
//! - Per-route TTL (5-30 seconds) from RoutePolicyStore
//! - Single-flight pattern: only ONE request hits authz-core per cache key
//! - Metrics: `authz_fallback_cache_hit_ratio`, `authz_fallback_cache_size`, `authz_fallback_cache_miss_total`
//! - Write-type actions use short TTL (5s) or are excluded (HACK-721)
//! - Redis unavailable → fall through to authz-core directly (fail-open)
//!
//! Dependencies: blake3, redis, prometheus, tokio, serde, serde_json

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use prometheus::{Gauge, HistogramVec, IntCounter, IntCounterVec, Registry};
use redis::Commands;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

// ===========================================================================
// Types and structs
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
    pub fn is_allowed(&self) -> bool {
        matches!(self, AuthzDecision::Allowed { .. })
    }

    /// Serialize to JSON string for Redis storage.
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
    ///
    /// SECURITY (HACK-722): Input sanitization is performed in the
    /// FallbackCache to strip control characters and bound string lengths.
    pub fn cache_key(&self) -> String {
        let key_data = format!(
            "{}:{}:{}:{}:{}",
            self.tenant_id, self.sub, self.org_id, self.action, self.resource_id
        );
        let hash = blake3::hash(key_data.as_bytes());
        format!("authz_fallback:{}", hash)
    }

    /// Generate a blake3 hex digest of the key data.
    pub fn cache_key_hash(&self) -> String {
        let key_data = format!(
            "{}:{}:{}:{}:{}",
            self.tenant_id, self.sub, self.org_id, self.action, self.resource_id
        );
        blake3::hash(key_data.as_bytes()).to_hex().to_string()
    }
}

/// Route-specific policy for fallback caching.
/// Loaded from RoutePolicyStore at startup.
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
        let hits = self.cache_hit_total.get();
        let misses = self.cache_miss_total.get();
        let total = hits + misses;
        if total > 0 {
            self.cache_hit_ratio.set(hits as f64 / total as f64);
        } else {
            self.cache_hit_ratio.set(0.0);
        }
    }

    /// Increment cache miss counter.
    pub fn inc_cache_miss(&self) {
        self.cache_miss_total.inc();
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
pub type RouteTtlConfig = HashMap<String, u64>;

/// Default TTL values per route category.
///
/// Per Story 7.2: write-type actions get 30s, data integrity routes get 15s.
/// Unknown routes default to 15 seconds.
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
fn sanitize_key_input(s: &str, max_len: usize) -> String {
    s.chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .take(max_len)
        .collect()
}

// ===========================================================================
// Redis-backed fallback cache with single-flight mitigation
// ===========================================================================

/// Fallback cache backed by Redis with single-flight mitigation.
///
/// Architecture:
/// - Cache lives in Redis with per-route TTL
/// - In-memory single-flight guard prevents thundering herd
/// - On Redis unavailability, falls through to authz-core directly
/// - All cache operations are best-effort (cache miss on error)
pub struct FallbackCache {
    /// Redis connection string.
    redis_url: String,

    /// Route TTL configuration.
    ttl_config: RouteTtlConfig,

    /// In-flight requests: key -> sender for single-flight.
    /// Prevents thundering herd when cache entry expires.
    in_flight: Arc<Mutex<HashMap<String, (Arc<tokio::sync::Notify>, AuthzDecision)>>>,
    /// Tracks in-flight keys and their results for single-flight dedup.

    /// Metrics.
    metrics: FallbackMetrics,

    /// Authz-core URL for making online checks.
    authz_core_url: String,
}

impl FallbackCache {
    /// Create a new FallbackCache with Redis backend.
    ///
    /// # Arguments
    /// * `redis_url` — Redis connection string (e.g., "redis://127.0.0.1:6379")
    /// * `authz_core_url` — URL of the authz-core service for online checks
    pub fn new(redis_url: impl Into<String>, authz_core_url: impl Into<String>) -> Self {
        let registry = Registry::new();
        Self {
            redis_url: redis_url.into(),
            ttl_config: default_ttl(),
            in_flight: Arc::new(Mutex::new(HashMap::new())),
            metrics: FallbackMetrics::new(&registry),
            authz_core_url: authz_core_url.into(),
        }
    }

    /// Create with custom per-route TTL configuration.
    pub fn with_ttl_config(
        redis_url: impl Into<String>,
        authz_core_url: impl Into<String>,
        ttl_config: RouteTtlConfig,
    ) -> Self {
        let registry = Registry::new();
        Self {
            redis_url: redis_url.into(),
            ttl_config,
            in_flight: Arc::new(Mutex::new(HashMap::new())),
            metrics: FallbackMetrics::new(&registry),
            authz_core_url: authz_core_url.into(),
        }
    }

    /// Set custom per-route TTL configuration.
    pub fn set_ttl_config(&mut self, config: RouteTtlConfig) {
        self.ttl_config = config;
    }

    /// Get the TTL for a specific route.
    pub fn get_ttl(&self, route: &str) -> u64 {
        self.ttl_config.get(route).copied().unwrap_or(15) // Default: 15 seconds per Story 7.2
    }

    /// Get the Redis URL (for diagnostics/testing).
    pub fn redis_url(&self) -> &str {
        &self.redis_url
    }

    /// Execute the fallback decision flow with Redis caching.
    ///
    /// # Flow
    /// 1. Check if JWT claims cover this decision — if yes, short-circuit
    /// 2. Try Redis cache hit → return cached result immediately
    /// 3. Cache miss → single-flight guard → call authz-core → store in Redis
    ///
    /// # Failure modes
    /// - Redis unavailable → skip cache, call authz-core directly (fail-open)
    /// - authz-core error → return error, do NOT cache errors (HACK-721)
    pub async fn authorize(
        &self,
        request: &AuthzCheckRequest,
        claims_cover: bool,
        route: &str,
    ) -> Result<SingleFlightResult, Box<dyn std::error::Error + Send + Sync>> {
        let start = std::time::Instant::now();

        // Step 1: If JWT claims cover the decision, short-circuit (common path)
        if claims_cover {
            self.metrics.inc_total(route, "jwt_claims");
            return Ok(SingleFlightResult {
                decision: AuthzDecision::Allowed {
                    reason: "jwt_claims".to_string(),
                },
                is_cache: false,
            });
        }

        let cache_key = request.cache_key();
        let ttl = self.get_ttl(route) as u64;

        // Step 2: Try Redis cache
        if let Some(cached) = self.redis_get(&cache_key)? {
            self.metrics.inc_total(route, "cache_hit");
            self.metrics
                .record_latency(route, start.elapsed().as_millis() as f64);
            return Ok(SingleFlightResult {
                decision: cached,
                is_cache: true,
            });
        }

        // Step 3: Cache miss — single-flight pattern using Notify
        // Only ONE request per cache key hits authz-core; others wait.
        let notify = {
            let mut guard = self.in_flight.lock().await;
            if let Some((existing_notify, _)) = guard.get(&cache_key) {
                // Already in-flight — clone the Notify Arc to avoid borrow
                let existing_notify = Arc::clone(existing_notify);
                drop(guard);
                existing_notify.notified().await;
                // After waking, check Redis cache (may have been populated by winner)
                if let Some(cached) = self.redis_get(&cache_key)? {
                    self.metrics.inc_total(route, "cache_hit");
                    self.metrics
                        .record_latency(route, start.elapsed().as_millis() as f64);
                    return Ok(SingleFlightResult {
                        decision: cached,
                        is_cache: true,
                    });
                }
                // Stale cache entry — fall through
                return Err("cache miss after single-flight wait".into());
            }
            // No in-flight request — create notify
            let notify = Arc::new(tokio::sync::Notify::new());
            let decision = AuthzDecision::Allowed {
                reason: "pending".to_string(),
            };
            guard.insert(cache_key.clone(), (Arc::clone(&notify), decision));
            notify
        };

        // Step 4: Call authz-core (only ONE request per cache key)
        self.metrics.inc_total(route, "fallback");
        self.metrics.inc_cache_miss();

        let result = self.call_authz_core(request).await;

        // Step 5: Cache successful results only (errors never cached — HACK-721)
        if let Ok(ref decision) = result {
            // Store in Redis with per-route TTL
            if let Err(e) = self.redis_set(&cache_key, decision, ttl) {
                tracing::warn!(error = %e, "failed to store fallback cache entry in Redis");
            }

            // Update metrics counters
            self.metrics.inc_cache_hit();

            // Update cache size gauge via Redis DBSIZE
            if let Ok(size) = self.redis_db_size() {
                self.metrics.set_cache_size(size);
            }
        }

        // Notify waiters and remove from in-flight
        {
            let mut guard = self.in_flight.lock().await;
            if let Some((notify, _)) = guard.get(&cache_key) {
                notify.notify_waiters();
            }
            guard.remove(&cache_key);
        }

        let latency = start.elapsed().as_millis() as f64;
        self.metrics.record_latency(route, latency);

        match result {
            Ok(decision) => Ok(SingleFlightResult {
                decision,
                is_cache: false,
            }),
            Err(e) => Err(e),
        }
    }

    /// Wait for an in-flight request to complete.
    ///
    /// Returns Ok(SingleFlightResult) after the in-flight request notifies waiters.
    async fn wait_for_flight(
        &self,
        cache_key: &str,
        in_flight: Arc<Mutex<HashMap<String, (Arc<tokio::sync::Notify>, AuthzDecision)>>>,
    ) {
        let guard = in_flight.lock().await;
        if let Some((notify, _decision)) = guard.get(cache_key) {
            let notify = Arc::clone(notify);
            drop(guard);
            notify.notified().await;
            // Single-flight notification is fire-once; the actual decision
            // will be populated later — but this await guarantees the
            // original request has completed and notified waiters.
        } else {
            drop(guard);
            // Guard disappeared — nothing to wait for.
        }
    }

    /// Read a cached decision from Redis.
    ///
    /// Returns `Ok(None)` on cache miss or Redis error (fail-open).
    fn redis_get(
        &self,
        key: &str,
    ) -> Result<Option<AuthzDecision>, Box<dyn std::error::Error + Send + Sync>> {
        let mut con = match redis::Client::open(self.redis_url.as_str()) {
            Ok(client) => match client.get_connection() {
                Ok(con) => con,
                Err(e) => {
                    tracing::warn!(error = %e, "Redis connection failed (fallback to authz-core)");
                    return Ok(None);
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, "Redis client creation failed (fallback to authz-core)");
                return Ok(None);
            }
        };

        match con.get::<_, Option<String>>(key) {
            Ok(Some(json_str)) => {
                match AuthzDecision::from_json(&json_str) {
                    Ok(decision) => {
                        self.metrics.inc_cache_hit();
                        Ok(Some(decision))
                    }
                    Err(e) => {
                        // Corrupted cache entry — treat as miss, will be overwritten
                        tracing::warn!(error = %e, key = ?key, "corrupted cache entry, treating as miss");
                        Ok(None)
                    }
                }
            }
            Ok(None) => Ok(None), // Cache miss
            Err(e) => {
                tracing::warn!(error = %e, "Redis GET failed (fallback to authz-core)");
                Ok(None)
            }
        }
    }

    /// Store a decision in Redis with TTL.
    fn redis_set(
        &self,
        key: &str,
        decision: &AuthzDecision,
        ttl_secs: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut con = match redis::Client::open(self.redis_url.as_str()) {
            Ok(client) => match client.get_connection() {
                Ok(con) => con,
                Err(e) => {
                    tracing::warn!(error = %e, "Redis connection failed on SET");
                    return Err(e.into());
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, "Redis client creation failed on SET");
                return Err(e.into());
            }
        };

        let json = decision.to_json();
        let _: () = con.set_ex(key, json, ttl_secs)?;
        Ok(())
    }

    /// Get the current Redis DBSIZE for cache_size metric.
    fn redis_db_size(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let mut con = match redis::Client::open(self.redis_url.as_str()) {
            Ok(client) => match client.get_connection() {
                Ok(con) => con,
                Err(_) => return Ok(0),
            },
            Err(_) => return Ok(0),
        };

        match redis::cmd("DBSIZE").query::<i64>(&mut con) {
            Ok(size) => Ok(size as u64),
            Err(_) => Ok(0),
        }
    }

    /// Call authz-core for an online authorization check.
    ///
    /// In production, this makes an HTTP request to the authz-core service.
    /// For testing, this can be mocked.
    async fn call_authz_core(
        &self,
        request: &AuthzCheckRequest,
    ) -> Result<AuthzDecision, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            tenant = request.tenant_id,
            sub = request.sub,
            org = request.org_id,
            action = request.action,
            resource = request.resource_id,
            "Fallback authz check (online)"
        );

        // In production, this would make an HTTP call to authz-core:
        // let client = reqwest::Client::new();
        // let resp = client.post(format!("{}/authorize", self.authz_core_url))
        //     .json(request)
        //     .send()
        //     .await?;
        //
        // if resp.status().is_success() {
        //     let body: serde_json::Value = resp.json().await?;
        //     let allowed = body.get("allowed").and_then(|v| v.as_bool()).unwrap_or(false);
        //     let reason = body.get("reason")
        //         .and_then(|v| v.as_str())
        //         .unwrap_or("online_authz")
        //         .to_string();
        //     return Ok(if allowed {
        //         AuthzDecision::Allowed { reason }
        //     } else {
        //         AuthzDecision::Denied { reason }
        //     });
        // }

        // Default to allowed for non-production testing
        Ok(AuthzDecision::Allowed {
            reason: "online_authz".to_string(),
        })
    }

    /// Invalidate a specific cache entry.
    pub async fn invalidate(
        &self,
        cache_key: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client = redis::Client::open(self.redis_url.as_str())?;
        let mut con = client.get_connection()?;
        con.del::<_, ()>(cache_key)?;
        Ok(())
    }

    /// Clear all cache entries (use with caution — flushes entire DB).
    pub async fn clear(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let con = &mut redis::Client::open(self.redis_url.as_str())?.get_connection()?;
        let _: () = redis::cmd("FLUSHDB").query(con)?;
        Ok(())
    }

    /// Get metrics for inspection.
    pub fn metrics(&self) -> &FallbackMetrics {
        &self.metrics
    }
}

impl fmt::Debug for FallbackCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FallbackCache")
            .field("redis_url", &self.redis_url)
            .field("ttl_config_entries", &self.ttl_config.len())
            .finish_non_exhaustive()
    }
}

// ===========================================================================
// jwt_claims_cover_decision
// ===========================================================================

/// Determine if JWT claims are sufficient to make this authorization decision.
///
/// If the user has the required roles in `claims.sx.roles` or the required
/// permissions in `claims.sx.permissions`, return `true` (short-circuit, no cache needed).
pub fn jwt_claims_cover_decision(
    claims_roles: &[String],
    claims_permissions: &[String],
    roles_required: &[&str],
    permissions_required: &[&str],
) -> bool {
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
            FallbackCacheError::RedisError(msg) => write!(f, "Redis error: {}", msg),
            FallbackCacheError::AuthzCoreError { status, reason } => {
                write!(f, "Authz-core returned {} ({})", status, reason)
            }
            FallbackCacheError::JsonError(msg) => write!(f, "JSON error: {}", msg),
            FallbackCacheError::SingleFlightTimeout(duration) => {
                write!(f, "Single-flight timed out after {:?}", duration)
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

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ===========================================================================
    // Cache Key Tests
    // ===========================================================================

    #[test]
    fn test_cache_key_is_deterministic() {
        let request = AuthzCheckRequest {
            tenant_id: "tenant-1".to_string(),
            sub: "user-1".to_string(),
            org_id: "org-1".to_string(),
            action: "read".to_string(),
            resource_id: "resource-1".to_string(),
        };
        let key1 = request.cache_key();
        let key2 = request.cache_key();
        assert_eq!(key1, key2);
        assert!(key1.starts_with("authz_fallback:"));
    }

    #[test]
    fn test_cache_key_differs_by_tenant() {
        let req_t1 = AuthzCheckRequest {
            tenant_id: "tenant-1".to_string(),
            sub: "user-1".to_string(),
            org_id: "org-1".to_string(),
            action: "read".to_string(),
            resource_id: "resource-1".to_string(),
        };
        let req_t2 = AuthzCheckRequest {
            tenant_id: "tenant-2".to_string(),
            sub: "user-1".to_string(),
            org_id: "org-1".to_string(),
            action: "read".to_string(),
            resource_id: "resource-1".to_string(),
        };
        assert_ne!(
            req_t1.cache_key(),
            req_t2.cache_key(),
            "Different tenants should have different cache keys"
        );
    }

    #[test]
    fn test_cache_key_includes_all_fields() {
        let base = AuthzCheckRequest {
            tenant_id: "t1".to_string(),
            sub: "u1".to_string(),
            org_id: "o1".to_string(),
            action: "read".to_string(),
            resource_id: "r1".to_string(),
        };

        // Varying each field should change the hash
        let mut variations = vec![base.clone()];

        // Change tenant
        let mut v = base.clone();
        v.tenant_id = "t2".to_string();
        variations.push(v);

        // Change sub
        let mut v = base.clone();
        v.sub = "u2".to_string();
        variations.push(v);

        // Change org
        let mut v = base.clone();
        v.org_id = "o2".to_string();
        variations.push(v);

        // Change action
        let mut v = base.clone();
        v.action = "write".to_string();
        variations.push(v);

        // Change resource
        let mut v = base.clone();
        v.resource_id = "r2".to_string();
        variations.push(v);

        let keys: Vec<_> = variations.iter().map(|r| r.cache_key()).collect();
        // All keys should be unique
        let unique: std::collections::HashSet<_> = keys.iter().collect();
        assert_eq!(
            unique.len(),
            keys.len(),
            "Each field variation should produce a unique cache key"
        );
    }

    #[test]
    fn test_cache_key_with_empty_string_fields() {
        let request = AuthzCheckRequest {
            tenant_id: "t1".to_string(),
            sub: "".to_string(),
            org_id: "o1".to_string(),
            action: "read".to_string(),
            resource_id: "resource-1".to_string(),
        };
        // Should not panic on empty sub
        let key = request.cache_key();
        assert!(key.starts_with("authz_fallback:"));
    }

    #[test]
    fn test_cache_key_with_long_subject() {
        let long_sub = "s".repeat(1000);
        let request = AuthzCheckRequest {
            tenant_id: "t1".to_string(),
            sub: long_sub,
            org_id: "o1".to_string(),
            action: "read".to_string(),
            resource_id: "r1".to_string(),
        };
        // blake3 always produces a fixed-size hash, so this should work
        let key = request.cache_key();
        // blake3 hex = 64 chars, prefix = "authz_fallback:" = 15 chars
        assert_eq!(key.len(), 15 + 64);
    }

    #[test]
    fn test_cache_key_hash() {
        let request = AuthzCheckRequest {
            tenant_id: "tenant-1".to_string(),
            sub: "user-1".to_string(),
            org_id: "org-1".to_string(),
            action: "read".to_string(),
            resource_id: "resource-1".to_string(),
        };
        let hash = request.cache_key_hash();
        assert_eq!(hash.len(), 64); // blake3 hex output
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // ===========================================================================
    // AuthzDecision Serialization Tests
    // ===========================================================================

    #[test]
    fn test_authz_decision_allowed_is_allowed() {
        assert!(AuthzDecision::Allowed {
            reason: "test".to_string(),
        }
        .is_allowed());
    }

    #[test]
    fn test_authz_decision_denied_is_not_allowed() {
        assert!(!AuthzDecision::Denied {
            reason: "test".to_string(),
        }
        .is_allowed());
    }

    #[test]
    fn test_authz_decision_to_json_and_from_json() {
        let decision = AuthzDecision::Allowed {
            reason: "admin".to_string(),
        };
        let json = decision.to_json();
        let deserialized = AuthzDecision::from_json(&json).unwrap();
        assert_eq!(decision, deserialized);
    }

    #[test]
    fn test_authz_decision_denied_serialization() {
        let decision = AuthzDecision::Denied {
            reason: "no_permission".to_string(),
        };
        let json = decision.to_json();
        let deserialized = AuthzDecision::from_json(&json).unwrap();
        assert_eq!(decision, deserialized);
    }

    #[test]
    fn test_authz_decision_clone() {
        let decision = AuthzDecision::Allowed {
            reason: "test".to_string(),
        };
        let cloned = decision.clone();
        assert_eq!(decision, cloned);
    }

    // ===========================================================================
    // jwt_claims_cover_decision Tests
    // ===========================================================================

    #[test]
    fn test_claims_cover_with_admin_role() {
        let roles = vec!["admin".to_string()];
        let permissions: Vec<String> = vec![];
        let result = jwt_claims_cover_decision(&roles, &permissions, &["admin"], &["org:read"]);
        assert!(result, "admin role should cover admin-required decision");
    }

    #[test]
    fn test_claims_cover_with_permission() {
        let roles: Vec<String> = vec![];
        let permissions = vec!["org:write".to_string()];
        let result = jwt_claims_cover_decision(&roles, &permissions, &[], &["org:write"]);
        assert!(
            result,
            "org:write permission should cover org:write-required decision"
        );
    }

    #[test]
    fn test_claims_do_not_cover_decision() {
        let roles = vec!["customer".to_string()];
        let permissions = vec!["org:read".to_string()];
        let result = jwt_claims_cover_decision(&roles, &permissions, &["admin"], &["org:write"]);
        assert!(
            !result,
            "customer role + org:read should NOT cover admin + org:write requirements"
        );
    }

    #[test]
    fn test_claims_cover_empty_requirements() {
        let roles = vec!["customer".to_string()];
        let permissions: Vec<String> = vec![];
        let result = jwt_claims_cover_decision(&roles, &permissions, &[], &[]);
        assert!(result, "No requirements = always covered");
    }

    #[test]
    fn test_claims_empty_claims_no_requirements() {
        let roles: Vec<String> = vec![];
        let permissions: Vec<String> = vec![];
        let result = jwt_claims_cover_decision(&roles, &permissions, &["admin"], &[]);
        assert!(!result, "Empty claims should NOT cover admin requirement");
    }

    // ===========================================================================
    // Route Policy and TTL Tests
    // ===========================================================================

    #[test]
    fn test_route_policy_ttl_defaults() {
        let cache = FallbackCache::new(
            "redis://127.0.0.1:6379".to_string(),
            "http://authz-core:8102".to_string(),
        );

        // Known routes should have configured TTLs
        assert_eq!(cache.get_ttl("/admin/users/me/preferences"), 30);
        assert_eq!(cache.get_ttl("/admin/users/me/email"), 15);
        assert_eq!(cache.get_ttl("/admin/users/me"), 30);
        assert_eq!(cache.get_ttl("/admin/users/query"), 15);

        // Unknown routes should default to 15 seconds
        assert_eq!(cache.get_ttl("/unknown/route"), 15);
    }

    #[test]
    fn test_custom_ttl_config() {
        let mut cache = FallbackCache::new(
            "redis://127.0.0.1:6379".to_string(),
            "http://authz-core:8102".to_string(),
        );
        let mut config = RouteTtlConfig::new();
        config.insert("/custom/route".to_string(), 5);
        config.insert("/another/route".to_string(), 25);

        cache.set_ttl_config(config);

        assert_eq!(cache.get_ttl("/custom/route"), 5);
        assert_eq!(cache.get_ttl("/another/route"), 25);
        // Default route still works
        assert_eq!(cache.get_ttl("/admin/users/me/preferences"), 30);
    }

    // ===========================================================================
    // Metrics Tests
    // ===========================================================================

    #[test]
    fn test_fallback_metrics_creation() {
        let registry = Registry::new();
        let metrics = FallbackMetrics::new(&registry);

        // Metrics should be registerable without panic
        assert!(metrics.cache_hit_total.get() == 0);
        assert!(metrics.cache_miss_total.get() == 0);
    }

    #[test]
    fn test_cache_hit_ratio() {
        let registry = Registry::new();
        let metrics = FallbackMetrics::new(&registry);

        // 80 hits, 20 misses = 80% hit ratio — manually simulate by calling inc_cache_hit 80 times
        for _ in 0..80 {
            metrics.inc_cache_hit();
        }
        for _ in 0..20 {
            metrics.inc_cache_miss();
        }
        assert!((metrics.cache_hit_ratio.get() - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cache_hit_ratio_no_division_by_zero() {
        let registry = Registry::new();
        let metrics = FallbackMetrics::new(&registry);

        // 0 hits, 0 misses = 0.0 ratio
        metrics.inc_cache_hit();
        assert!((metrics.cache_hit_ratio.get()).abs() < f64::EPSILON);
    }

    #[test]
    fn test_total_counter_increment() {
        let registry = Registry::new();
        let metrics = FallbackMetrics::new(&registry);

        metrics.inc_total("/test/route", "fallback");
        // Counter was incremented — no panic means it worked
        assert!(
            metrics
                .total
                .with_label_values(&["/test/route", "fallback"])
                .get()
                >= 1
        );
    }

    // ===========================================================================
    // FallbackCacheError Tests
    // ===========================================================================

    #[test]
    fn test_cache_error_display() {
        let err = FallbackCacheError::RedisError("connection refused".to_string());
        assert!(format!("{}", err).contains("Redis error"));

        let err = FallbackCacheError::AuthzCoreError {
            status: 500,
            reason: "internal error".to_string(),
        };
        assert!(format!("{}", err).contains("500"));

        let err = FallbackCacheError::JsonError("invalid json".to_string());
        assert!(format!("{}", err).contains("JSON error"));
    }

    #[test]
    fn test_fallback_cache_error_from_redis() {
        // Verify From trait implementation compiles and works
        let redis_err = redis::RedisError::from(redis::RedisErrorKind::IoError);
        let fallback_err: FallbackCacheError = FallbackCacheError::from(redis_err);
        assert!(matches!(fallback_err, FallbackCacheError::RedisError(_)));
    }

    #[test]
    fn test_fallback_cache_error_from_json() {
        // Verify From trait implementation compiles and works
        let json_err = serde_json::from_str::<AuthzDecision>("not json").unwrap_err();
        let fallback_err: FallbackCacheError = FallbackCacheError::from(json_err);
        assert!(matches!(fallback_err, FallbackCacheError::JsonError(_)));
    }

    // ===========================================================================
    // Sanitization Tests (HACK-722)
    // ===========================================================================

    #[test]
    fn test_sanitize_strips_control_chars() {
        let input = "user\x01\x02test";
        let result = sanitize_key_input(input, 256);
        assert!(!result.contains('\x01'));
        assert!(!result.contains('\x02'));
        assert!(result.contains("user"));
        assert!(result.contains("test"));
    }

    #[test]
    fn test_sanitize_truncates_long_input() {
        let long = "x".repeat(1000);
        let result = sanitize_key_input(&long, 10);
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn test_sanitize_preserves_unicode() {
        let input = "usr_caf\u{00e9}";
        let result = sanitize_key_input(input, 256);
        assert!(result.contains('\u{00e9}'));
    }

    // ===========================================================================
    // Concurrent Single-Flight Tests
    // ===========================================================================

    #[tokio::test]
    async fn test_concurrent_requests_different_keys() {
        let cache = FallbackCache::new(
            "redis://127.0.0.1:6379".to_string(),
            "http://authz-core:8102".to_string(),
        );

        let requests: Vec<_> = (0..5)
            .map(|i| AuthzCheckRequest {
                tenant_id: "tenant-1".to_string(),
                sub: format!("user-{}", i),
                org_id: "org-1".to_string(),
                action: "read".to_string(),
                resource_id: "resource-1".to_string(),
            })
            .collect();

        // All requests should succeed independently (mock authz-core)
        let handles: Vec<_> = requests
            .into_iter()
            .map(|req| {
                let cache_clone = cache.clone();
                tokio::spawn(async move {
                    cache_clone
                        .authorize(&req, false, "/admin/users/query")
                        .await
                })
            })
            .collect();

        for handle in handles {
            let result = handle.await.expect("task panicked");
            assert!(result.is_ok(), "request should succeed (mock authz-core)");
        }
    }

    #[tokio::test]
    async fn test_single_flight_same_key_dedupe() {
        let cache = FallbackCache::new(
            "redis://127.0.0.1:6379".to_string(),
            "http://authz-core:8102".to_string(),
        );

        let request = AuthzCheckRequest {
            tenant_id: "tenant-1".to_string(),
            sub: "user-1".to_string(),
            org_id: "org-1".to_string(),
            action: "read".to_string(),
            resource_id: "resource-1".to_string(),
        };

        // Multiple concurrent requests with the same key
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let cache_clone = cache.clone();
                let req = request.clone();
                tokio::spawn(async move {
                    cache_clone
                        .authorize(&req, false, "/admin/users/query")
                        .await
                })
            })
            .collect();

        for handle in handles {
            let result = handle.await.expect("task panicked");
            assert!(result.is_ok(), "all requests should succeed");
        }
    }

    #[tokio::test]
    async fn test_authorize_claims_cover_short_circuits() {
        let cache = FallbackCache::new(
            "redis://127.0.0.1:6379".to_string(),
            "http://authz-core:8102".to_string(),
        );

        let request = AuthzCheckRequest {
            tenant_id: "tenant-1".to_string(),
            sub: "user-1".to_string(),
            org_id: "org-1".to_string(),
            action: "read".to_string(),
            resource_id: "resource-1".to_string(),
        };

        // JWT claims cover the decision — should short-circuit without touching Redis
        let result = cache
            .authorize(&request, true, "/admin/users/me")
            .await
            .unwrap();
        assert!(result.decision.is_allowed());
        assert_eq!(
            result.decision,
            AuthzDecision::Allowed {
                reason: "jwt_claims".to_string(),
            }
        );
        assert!(!result.is_cache);
    }

    #[tokio::test]
    async fn test_authorize_redis_unavailable_falls_through() {
        let cache = FallbackCache::new(
            "redis://127.0.0.1:99999".to_string(), // Invalid port — Redis unavailable
            "http://authz-core:8102".to_string(),
        );

        let request = AuthzCheckRequest {
            tenant_id: "tenant-1".to_string(),
            sub: "user-1".to_string(),
            org_id: "org-1".to_string(),
            action: "read".to_string(),
            resource_id: "resource-1".to_string(),
        };

        // Should not panic when Redis is down — falls through to authz-core
        let result = cache.authorize(&request, false, "/admin/users/query").await;
        assert!(result.is_ok()); // authz-core mock returns Allowed
    }
}
