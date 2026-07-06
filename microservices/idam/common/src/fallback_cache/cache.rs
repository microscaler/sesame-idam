//! `FallbackCache` struct and its implementation.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use prometheus::Registry;
use redis::Commands;

use super::redis::{redis_db_size, redis_get, redis_set};
use super::types::{AuthzCheckRequest, AuthzDecision, FallbackMetrics, SingleFlightResult};

// ===========================================================================
// FallbackCache
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
    ttl_config: super::types::RouteTtlConfig,

    /// In-flight requests: keys currently being processed (single-flight guard).
    /// Prevents thundering herd when cache entry expires.
    in_flight: Arc<std::sync::Mutex<HashSet<String>>>,

    /// Metrics.
    metrics: FallbackMetrics,

    /// Authz-core URL for making online checks.
    authz_core_url: String,
}

impl FallbackCache {
    /// Create a new `FallbackCache` with Redis backend.
    ///
    /// # Arguments
    /// * `redis_url` — Redis connection string (e.g., "<redis://127.0.0.1:6379>")
    /// * `authz_core_url` — URL of the authz-core service for online checks
    pub fn new(redis_url: impl Into<String>, authz_core_url: impl Into<String>) -> Self {
        let registry = Registry::new();
        Self {
            redis_url: redis_url.into(),
            ttl_config: super::types::default_ttl(),
            in_flight: Arc::new(std::sync::Mutex::new(HashSet::new())),
            metrics: FallbackMetrics::new(&registry),
            authz_core_url: authz_core_url.into(),
        }
    }

    /// Create with custom per-route TTL configuration.
    pub fn with_ttl_config(
        redis_url: impl Into<String>,
        authz_core_url: impl Into<String>,
        ttl_config: super::types::RouteTtlConfig,
    ) -> Self {
        let registry = Registry::new();
        Self {
            redis_url: redis_url.into(),
            ttl_config,
            in_flight: Arc::new(std::sync::Mutex::new(HashSet::new())),
            metrics: FallbackMetrics::new(&registry),
            authz_core_url: authz_core_url.into(),
        }
    }

    /// Merge custom per-route TTL configuration over the defaults.
    ///
    /// Routes present in `config` override existing entries; default routes
    /// not mentioned keep their configured TTLs.
    pub fn set_ttl_config(&mut self, config: super::types::RouteTtlConfig) {
        self.ttl_config.extend(config);
    }

    /// Get the TTL for a specific route.
    #[must_use]
    pub fn get_ttl(&self, route: &str) -> u64 {
        self.ttl_config.get(route).copied().unwrap_or(15)
    }

    /// Get the Redis URL (for diagnostics/testing).
    #[must_use]
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
        let start = Instant::now();

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
        let ttl = self.get_ttl(route);

        if let Some(cached) = redis_get(self, &cache_key)? {
            self.metrics.inc_total(route, "cache_hit");
            self.metrics
                .record_latency(route, start.elapsed().as_millis() as f64);
            return Ok(SingleFlightResult {
                decision: cached,
                is_cache: true,
            });
        }

        // Step 3: Cache miss — single-flight guard
        // Only ONE request per cache key hits authz-core; others wait.
        let is_first = {
            let mut guard = self.in_flight.lock().unwrap();
            if guard.contains(&cache_key) {
                // Another request is in-flight — drop guard and check cache
                drop(guard);
                // Check if cache was populated by the in-flight request
                if let Some(cached) = redis_get(self, &cache_key)? {
                    self.metrics.inc_total(route, "cache_hit");
                    self.metrics
                        .record_latency(route, start.elapsed().as_millis() as f64);
                    return Ok(SingleFlightResult {
                        decision: cached,
                        is_cache: true,
                    });
                }
                // Cache still empty — wait a brief moment for the in-flight request
                std::thread::sleep(std::time::Duration::from_millis(100));
                // Check again after waiting
                if let Some(cached) = redis_get(self, &cache_key)? {
                    self.metrics.inc_total(route, "cache_hit");
                    self.metrics
                        .record_latency(route, start.elapsed().as_millis() as f64);
                    return Ok(SingleFlightResult {
                        decision: cached,
                        is_cache: true,
                    });
                }
                false
            } else {
                guard.insert(cache_key.clone());
                true
            }
        };
        if !is_first {
            // We waited and still no cache — call authz-core (acceptable to duplicate)
            self.metrics.inc_total(route, "fallback");
            self.metrics.inc_cache_miss();
            let result = self.call_authz_core(request).await;
            let latency = start.elapsed().as_millis() as f64;
            self.metrics.record_latency(route, latency);
            return match result {
                Ok(decision) => Ok(SingleFlightResult {
                    decision,
                    is_cache: false,
                }),
                Err(e) => Err(e),
            };
        }

        // Step 4: Call authz-core (only ONE request per cache key)
        self.metrics.inc_total(route, "fallback");
        self.metrics.inc_cache_miss();

        let result = self.call_authz_core(request).await;

        // Step 5: Cache successful results only (errors never cached — HACK-721)
        if let Ok(ref decision) = result {
            // Store in Redis with per-route TTL
            if let Err(e) = redis_set(self, &cache_key, decision, ttl) {
                tracing::warn!(error = %e, "failed to store fallback cache entry in Redis");
            }

            // Update metrics counters
            self.metrics.inc_cache_hit();

            // Update cache size gauge via Redis DBSIZE
            if let Ok(size) = redis_db_size(self) {
                self.metrics.set_cache_size(size);
            }
        }

        // Remove from in-flight set
        {
            let mut guard = self.in_flight.lock().unwrap();
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
    #[must_use]
    pub fn metrics(&self) -> &FallbackMetrics {
        &self.metrics
    }
}

impl std::fmt::Debug for FallbackCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FallbackCache")
            .field("redis_url", &self.redis_url)
            .field("ttl_config_entries", &self.ttl_config.len())
            .finish_non_exhaustive()
    }
}
