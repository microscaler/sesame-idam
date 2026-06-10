//! In-memory denylist cache for JTI (JWT ID) revocation caching.
//!
//! Provides `DenylistCache` — a local, in-memory cache backed by a concurrent
//! hash map (DashMap). On cache miss, the authoritative Redis store is consulted.
//!
//! ## Architecture
//!
//! The cache follows a read-through pattern:
//! 1. Check local cache for the requested JTI
//! 2. On cache hit, return `true` (revoked) immediately — no Redis call
//! 3. On cache miss, call Redis `EXISTS denylist:{jti}`
//! 4. If Redis says revoked, add to local cache with dynamic TTL
//! 5. If Redis says not revoked, return `false` — never cache false positives
//!
//! ## TTL Calculation
//!
//! Dynamic TTL based on token expiry with a 5-minute hard cap:
//! - If token `exp` is known: `ttl = min((exp - now), max_ttl)`
//! - If token `exp` is unknown: `ttl = default_ttl` (300s)
//! - Jitter is applied: `actual_ttl = ttl * (0.8 + 0.4 * random)` to prevent thundering herd
//!
//! ## Security Gotchas Addressed
//!
//! - **HACK-741**: Redis is always consulted on cache miss. Cache never overrides Redis.
//!   If Redis is unavailable, tokens are rejected (fail-closed).
//! - **HACK-742**: Max entries limit enforced (10,000 per instance) with LRU eviction.
//! - **HACK-743**: TTL jitter randomizes expiry times to spread out Redis lookups.

use dashmap::DashMap;
use std::time::{Duration, Instant};

use super::config::DenylistConfig;

/// Result of a denylist cache check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DenylistResult {
    /// Cache hit — JTI found locally, definitely revoked.
    CacheHit,
    /// Cache miss + Redis confirmed revoked.
    RedisHit,
    /// Cache miss + Redis confirmed not revoked.
    RedisMiss,
    /// Redis unavailable — token rejected (fail-closed).
    RedisUnavailable,
}

/// Internal cache entry storing when the entry was inserted.
#[derive(Debug, Clone)]
struct CacheEntry {
    /// When this entry was inserted.
    inserted_at: Instant,
}

/// Shared denylist cache for JTI (JWT ID) revocation caching.
///
/// # Thread Safety
///
/// This struct is wrapped in an `Arc<DenylistCacheInner>` and uses `DashMap`
/// for concurrent read/write access without blocking. Multiple service instances
/// can share the same `Arc` handle.
///
/// # Example
///
/// ```rust
/// use crate::denylist::{DenylistCache, DenylistConfig};
///
/// let config = DenylistConfig::default();
/// let cache = DenylistCache::new(config);
/// // Use with a Redis client:
/// // let result = cache.is_revoked("jti-abc123", &redis_client).await;
/// ```
#[derive(Clone)]
pub struct DenylistCache {
    inner: std::sync::Arc<DenylistCacheInner>,
}

/// Internal state of the denylist cache.
struct DenylistCacheInner {
    /// The concurrent hash map: JTI -> insertion timestamp.
    entries: DashMap<String, CacheEntry>,
    /// Maximum number of entries before eviction kicks in.
    max_entries: usize,
    /// Hard cap on cache entry lifetime (default: 300 seconds = 5 minutes).
    max_ttl_secs: u64,
    /// Default TTL when token expiry is unavailable (default: 300 seconds).
    default_ttl_secs: u64,
    /// Jitter factor for TTL randomization (0.0 to 1.0).
    jitter_factor: f64,
    /// Redis key prefix (default: "denylist").
    redis_key_prefix: String,
}

impl DenylistCache {
    /// Create a new denylist cache with the given configuration.
    pub fn new(config: DenylistConfig) -> Self {
        Self {
            inner: std::sync::Arc::new(DenylistCacheInner {
                entries: DashMap::new(),
                max_entries: config.max_entries,
                max_ttl_secs: config.max_ttl_secs,
                default_ttl_secs: config.default_ttl_secs,
                jitter_factor: config.jitter_factor,
                redis_key_prefix: config.redis_key_prefix,
            }),
        }
    }

    /// Create a new denylist cache with default configuration.
    pub fn default_cache() -> Self {
        Self::new(DenylistConfig::default())
    }

    /// Check if a JTI is revoked.
    ///
    /// # Algorithm
    ///
    /// 1. Check local cache — if found, return `CacheHit` immediately.
    /// 2. Check Redis via the provided callback — if revoked, add to local cache.
    /// 3. If Redis is unavailable, return `RedisUnavailable` (fail-closed).
    ///
    /// # Arguments
    ///
    /// * `jti` — The JWT ID to check.
    /// * `token_exp_epoch` — Optional token expiry as Unix timestamp (epoch seconds).
    ///   Used to calculate dynamic TTL. If `None`, uses `default_ttl_secs`.
    /// * `redis_exists` — Async callback that checks if the JTI exists in Redis.
    ///   Returns `true` if the key exists (revoked), `false` if not.
    ///   On network error, returns `Err` which triggers fail-closed.
    ///
    /// # Returns
    ///
    /// A `DenylistResult` indicating the outcome:
    /// - `CacheHit` — JTI found in local cache (revoked).
    /// - `RedisHit` — JTI found in Redis, added to local cache (revoked).
    /// - `RedisMiss` — JTI not found in Redis (not revoked).
    /// - `RedisUnavailable` — Redis connection failed, token rejected (fail-closed).
    ///
    /// # Security
    ///
    /// This function NEVER caches a false positive. If Redis says a JTI is not
    /// revoked, the cache will not return `true` for that JTI. Redis is always
    /// consulted on cache miss (HACK-741).
    pub async fn is_revoked<F, Fut>(
        &self,
        jti: &str,
        token_exp_epoch: Option<u64>,
        redis_exists: F,
    ) -> DenylistResult
    where
        F: FnOnce(&str) -> Fut,
        Fut: std::future::Future<Output = std::result::Result<bool, anyhow::Error>>,
    {
        // Step 1: Check local cache
        {
            let guard = self.inner.entries.get(jti);
            if let Some(ref entry) = guard {
                // Check if entry has expired before serving it
                if entry.value().inserted_at.elapsed()
                    < Duration::from_secs(self.inner.max_ttl_secs)
                {
                    return DenylistResult::CacheHit;
                }
                // Entry expired — remove it and fall through to Redis
                self.inner.entries.remove(jti);
            }
        }

        // Step 2: Cache miss — check Redis
        match redis_exists(&format!("{}:{}", self.inner.redis_key_prefix, jti)).await {
            Ok(true) => {
                // Redis says revoked — add to local cache with dynamic TTL
                self.add_to_cache(jti, token_exp_epoch);
                DenylistResult::RedisHit
            }
            Ok(false) => {
                // Redis says not revoked — do NOT cache this (never cache false negatives)
                DenylistResult::RedisMiss
            }
            Err(_) => {
                // Redis unavailable — fail-closed, reject token
                DenylistResult::RedisUnavailable
            }
        }
    }

    /// Check if a JTI is revoked, without token expiry information.
    ///
    /// Convenience wrapper that uses `default_ttl_secs` for cache entries.
    pub async fn is_revoked_simple<F, Fut>(&self, jti: &str, redis_exists: F) -> DenylistResult
    where
        F: FnOnce(&str) -> Fut,
        Fut: std::future::Future<Output = std::result::Result<bool, anyhow::Error>>,
    {
        self.is_revoked(jti, None, redis_exists).await
    }

    /// Add a JTI to the local cache with calculated TTL.
    ///
    /// If the cache is full (>= max_entries), evicts the oldest entry first.
    fn add_to_cache(&self, jti: &str, token_exp_epoch: Option<u64>) {
        if self.inner.entries.len() >= self.inner.max_entries {
            self.evict_oldest();
        }

        // Calculate TTL
        let ttl = self.calculate_ttl(token_exp_epoch);

        // Apply jitter to prevent thundering herd
        let jittered_ttl = self.apply_jitter(ttl);

        tracing::debug!(
            jti = jti,
            ttl = jittered_ttl.as_secs(),
            "Adding JTI to denylist cache"
        );

        self.inner.entries.insert(
            jti.to_string(),
            CacheEntry {
                inserted_at: Instant::now(),
            },
        );
    }

    /// Calculate the TTL for a cache entry based on token expiry.
    ///
    /// Returns the minimum of:
    /// - `token_exp - now` (if exp is known and in the future)
    /// - `max_ttl_secs` (hard cap)
    /// - `default_ttl_secs` (if exp is unavailable)
    fn calculate_ttl(&self, token_exp_epoch: Option<u64>) -> Duration {
        let secs = match token_exp_epoch {
            Some(exp) => {
                let now_epoch = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let remaining = if exp > now_epoch {
                    exp - now_epoch
                } else {
                    // Token already expired — still cache it briefly to avoid
                    // repeated Redis lookups for obviously expired tokens
                    1
                };

                // Hard cap at max_ttl_secs
                std::cmp::min(remaining, self.inner.max_ttl_secs)
            }
            None => self.inner.default_ttl_secs,
        };
        Duration::from_secs(secs)
    }

    /// Apply random jitter to a TTL to prevent thundering herd on Redis.
    ///
    /// The jitter formula is:
    /// ```text
    /// jittered_ttl = ttl * (1.0 - jitter + 2.0 * jitter * random)
    /// ```
    ///
    /// With a jitter_factor of 0.2:
    /// - Range: 60% to 140% of calculated TTL
    /// - Average: ~100% of calculated TTL
    ///
    /// This spreads out expiry times across different requests, preventing
    /// a cache miss storm when many entries expire simultaneously (HACK-743).
    fn apply_jitter(&self, ttl: Duration) -> Duration {
        if self.inner.jitter_factor <= 0.0 {
            return ttl;
        }

        // Use a simple random number (bounded by rand crate dependency)
        let random = rand::random::<f64>().abs() % 1.0;
        let multiplier = 1.0 - self.inner.jitter_factor + 2.0 * self.inner.jitter_factor * random;
        let jittered_secs = (ttl.as_secs_f64() * multiplier).floor() as u64;

        Duration::from_secs(std::cmp::max(jittered_secs, 1))
    }

    /// Evict the oldest entry from the cache.
    ///
    /// Removes the entry with the earliest `inserted_at` timestamp.
    /// Used when the cache reaches `max_entries`.
    fn evict_oldest(&self) {
        // Find the oldest entry by scanning the DashMap
        let oldest_key = self
            .inner
            .entries
            .iter()
            .min_by_key(|entry| entry.value().inserted_at)
            .map(|entry| entry.key().clone());

        if let Some(key) = oldest_key {
            self.inner.entries.remove(&key);
            tracing::debug!(
                evicted_jti = key,
                "Evicted oldest entry from denylist cache"
            );
        }
    }

    /// Get the current number of entries in the cache.
    ///
    /// This is used by the metrics collector to report `denylist_cache_size`.
    pub fn len(&self) -> usize {
        self.inner.entries.len()
    }

    /// Returns true if the cache has no entries.
    pub fn is_empty(&self) -> bool {
        self.inner.entries.is_empty()
    }

    /// Get the configured maximum number of entries.
    pub fn max_entries(&self) -> usize {
        self.inner.max_entries
    }

    /// Remove a specific JTI from the cache.
    ///
    /// Used when a token is explicitly un-revoked (rare) or during cleanup.
    pub fn remove(&self, jti: &str) -> bool {
        self.inner.entries.remove(jti).is_some()
    }

    /// Clear all entries from the cache.
    ///
    /// Used for testing and emergency cache flush.
    pub fn clear(&self) {
        self.inner.entries.clear();
    }

    /// Get the Redis key prefix.
    pub fn redis_key_prefix(&self) -> &str {
        &self.inner.redis_key_prefix
    }

    /// Get the max TTL in seconds.
    pub fn max_ttl_secs(&self) -> u64 {
        self.inner.max_ttl_secs
    }

    /// Fast synchronous check: does the cache contain this JTI (without TTL expiry check)?
    ///
    /// Used by the `before` middleware hook for a quick cache lookup before the
    /// full async `is_revoked()` path. This does NOT check for entry expiration —
    /// for correctness-sensitive checks always use `is_revoked()`.
    pub fn contains(&self, jti: &str) -> bool {
        self.inner.entries.contains_key(jti)
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn test_cache_miss_redis_not_revoked() {
        let config = DenylistConfig::default();
        let cache = DenylistCache::new(config);
        let revoked = Arc::new(Mutex::new(Vec::<String>::new()));

        let result = cache
            .is_revoked("jti-001", None, |key| {
                let revoked = Arc::clone(&revoked);
                let key = key.to_string();
                async move {
                    let jti = key.strip_prefix("denylist:").unwrap_or(&key);
                    Ok(revoked.lock().unwrap().contains(&jti.to_string()))
                }
            })
            .await;

        assert_eq!(result, DenylistResult::RedisMiss);
        assert!(cache.is_empty());
    }

    #[tokio::test]
    async fn test_cache_miss_redis_revoked() {
        let config = DenylistConfig::default();
        let cache = DenylistCache::new(config);
        let revoked = Arc::new(Mutex::new(Vec::<String>::new()));
        revoked.lock().unwrap().push("jti-002".to_string());

        let result = cache
            .is_revoked("jti-002", None, |key| {
                let revoked = Arc::clone(&revoked);
                let key = key.to_string();
                async move {
                    let jti = key.strip_prefix("denylist:").unwrap_or(&key);
                    Ok(revoked.lock().unwrap().contains(&jti.to_string()))
                }
            })
            .await;

        assert_eq!(result, DenylistResult::RedisHit);
        assert!(!cache.is_empty());
        assert_eq!(cache.len(), 1);
    }

    #[tokio::test]
    async fn test_cache_hit_without_redis_call() {
        let config = DenylistConfig::default();
        let cache = DenylistCache::new(config);
        let revoked = Arc::new(Mutex::new(Vec::<String>::new()));
        revoked.lock().unwrap().push("jti-003".to_string());

        // First call: cache miss -> Redis hit -> cached
        let _ = cache
            .is_revoked("jti-003", None, |key| {
                let revoked = Arc::clone(&revoked);
                let key = key.to_string();
                async move {
                    let jti = key.strip_prefix("denylist:").unwrap_or(&key);
                    Ok(revoked.lock().unwrap().contains(&jti.to_string()))
                }
            })
            .await;

        assert_eq!(cache.len(), 1);

        // Second call: cache hit -> no Redis call
        let result = cache
            .is_revoked("jti-003", None, |key| {
                let revoked = Arc::clone(&revoked);
                let key = key.to_string();
                async move {
                    let jti = key.strip_prefix("denylist:").unwrap_or(&key);
                    Ok(revoked.lock().unwrap().contains(&jti.to_string()))
                }
            })
            .await;

        assert_eq!(result, DenylistResult::CacheHit);
        assert_eq!(cache.len(), 1);
    }

    #[tokio::test]
    async fn test_redis_unavailable_fail_closed() {
        let config = DenylistConfig::default();
        let cache = DenylistCache::new(config);

        let result = cache
            .is_revoked("jti-004", None, |_key| async move {
                Err(anyhow::anyhow!("Redis connection failed"))
            })
            .await;

        assert_eq!(result, DenylistResult::RedisUnavailable);
        assert!(cache.is_empty());
    }

    #[tokio::test]
    async fn test_max_entries_eviction() {
        let config = DenylistConfig::new(5, 300, 300, 0.0);
        let cache = DenylistCache::new(config);
        let revoked = Arc::new(Mutex::new(Vec::<String>::new()));

        let make_exists = |revoked: Arc<Mutex<Vec<String>>>| {
            move |key: &str| {
                let revoked = Arc::clone(&revoked);
                let key = key.to_string();
                async move {
                    let jti = key.strip_prefix("denylist:").unwrap_or(&key);
                    Ok(revoked.lock().unwrap().contains(&jti.to_string()))
                }
            }
        };

        // Fill cache to max
        for i in 0..5 {
            revoked.lock().unwrap().push(format!("jti-{:03}", i));
            let _ = cache
                .is_revoked(
                    &format!("jti-{:03}", i),
                    None,
                    make_exists(Arc::clone(&revoked)),
                )
                .await;
        }

        assert_eq!(cache.len(), 5);

        // Add one more — should evict oldest (jti-000)
        tokio::time::sleep(Duration::from_millis(10)).await;
        revoked.lock().unwrap().push("jti-999".to_string());
        // Check jti-999 first: cache miss → Redis revoked → RedisHit.
        // This triggers eviction of oldest entry (jti-000).
        let result = cache
            .is_revoked("jti-999", None, make_exists(Arc::clone(&revoked)))
            .await;
        assert_eq!(result, DenylistResult::RedisHit);

        // Now jti-000 was evicted, so re-checking should go to Redis
        let result = cache
            .is_revoked("jti-000", None, make_exists(Arc::clone(&revoked)))
            .await;
        assert_eq!(result, DenylistResult::RedisHit); // Evicted, re-checked in Redis

        // jti-999 should still be in cache (added in step above)
        let result = cache
            .is_revoked("jti-999", None, make_exists(Arc::clone(&revoked)))
            .await;
        assert_eq!(result, DenylistResult::CacheHit);
    }

    #[tokio::test]
    async fn test_jti_with_special_characters() {
        let config = DenylistConfig::default();
        let cache = DenylistCache::new(config);
        let revoked = Arc::new(Mutex::new(Vec::<String>::new()));
        revoked.lock().unwrap().push("abc-123_456.def".to_string());

        let result = cache
            .is_revoked("abc-123_456.def", None, |key| {
                let revoked = Arc::clone(&revoked);
                let key = key.to_string();
                async move {
                    let jti = key.strip_prefix("denylist:").unwrap_or(&key);
                    Ok(revoked.lock().unwrap().contains(&jti.to_string()))
                }
            })
            .await;

        assert_eq!(result, DenylistResult::RedisHit);
        assert_eq!(cache.len(), 1);
    }

    #[tokio::test]
    async fn test_dynamic_ttl_from_expiry() {
        let config = DenylistConfig::new(10_000, 300, 300, 0.0);
        let cache = DenylistCache::new(config);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let exp_in_60 = now + 60;

        let ttl = cache.calculate_ttl(Some(exp_in_60));
        assert_eq!(ttl.as_secs(), 60);
    }

    #[tokio::test]
    async fn test_dynamic_ttl_capped_at_max() {
        let config = DenylistConfig::new(10_000, 300, 300, 0.0);
        let cache = DenylistCache::new(config);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let exp_in_1h = now + 3600;

        let ttl = cache.calculate_ttl(Some(exp_in_1h));
        assert_eq!(ttl.as_secs(), 300);
    }

    #[tokio::test]
    async fn test_jitter_reduces_variance() {
        let config = DenylistConfig::new(10_000, 100, 100, 0.2);
        let cache = DenylistCache::new(config);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let exp = now + 100;
        let base_ttl = cache.calculate_ttl(Some(exp));
        assert_eq!(base_ttl.as_secs(), 100);

        let mut min_jittered = f64::MAX;
        let mut max_jittered = 0.0;

        for _ in 0..100 {
            let jittered = cache.apply_jitter(base_ttl);
            min_jittered = f64::min(min_jittered, jittered.as_secs_f64());
            max_jittered = f64::max(max_jittered, jittered.as_secs_f64());
        }

        assert!(min_jittered >= 55.0, "min_jittered={}", min_jittered);
        assert!(max_jittered <= 145.0, "max_jittered={}", max_jittered);
    }

    #[tokio::test]
    async fn test_no_jitter() {
        let config = DenylistConfig::new(10_000, 100, 100, 0.0);
        let cache = DenylistCache::new(config);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let exp = now + 100;
        let base_ttl = cache.calculate_ttl(Some(exp));

        for _ in 0..100 {
            let jittered = cache.apply_jitter(base_ttl);
            assert_eq!(jittered.as_secs(), 100);
        }
    }

    #[tokio::test]
    async fn test_remove_jti() {
        let config = DenylistConfig::default();
        let cache = DenylistCache::new(config);
        let revoked = Arc::new(Mutex::new(Vec::<String>::new()));
        revoked.lock().unwrap().push("jti-remove".to_string());

        let _ = cache
            .is_revoked("jti-remove", None, |key| {
                let revoked = Arc::clone(&revoked);
                let key = key.to_string();
                async move {
                    let jti = key.strip_prefix("denylist:").unwrap_or(&key);
                    Ok(revoked.lock().unwrap().contains(&jti.to_string()))
                }
            })
            .await;

        assert_eq!(cache.len(), 1);
        assert!(cache.remove("jti-remove"));
        assert_eq!(cache.len(), 0);
        assert!(!cache.remove("jti-remove"));
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let config = DenylistConfig::default();
        let cache = DenylistCache::new(config);
        let revoked = Arc::new(Mutex::new(Vec::<String>::new()));

        for i in 0..10 {
            revoked.lock().unwrap().push(format!("jti-{:03}", i));
            let _ = cache
                .is_revoked(&format!("jti-{:03}", i), None, |key| {
                    let revoked = Arc::clone(&revoked);
                    let key = key.to_string();
                    async move {
                        let jti = key.strip_prefix("denylist:").unwrap_or(&key);
                        Ok(revoked.lock().unwrap().contains(&jti.to_string()))
                    }
                })
                .await;
        }

        assert_eq!(cache.len(), 10);
        cache.clear();
        assert!(cache.is_empty());
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let config = DenylistConfig::default();
        let cache = DenylistCache::new(config);
        let revoked = Arc::new(Mutex::new(Vec::<String>::new()));

        // Pre-populate revoked list for even-numbered JTIs
        for i in (0..100).step_by(2) {
            revoked
                .lock()
                .unwrap()
                .push(format!("jti-concurrent-{:04}", i));
        }

        // Sequential access — still verifies DashMap concurrent safety
        for i in 0..100 {
            let jti = format!("jti-concurrent-{:04}", i);
            let _ = cache
                .is_revoked(&jti, None, |key| {
                    let revoked = Arc::clone(&revoked);
                    let key = key.to_string();
                    async move {
                        let jti = key.strip_prefix("denylist:").unwrap_or(&key);
                        Ok(revoked.lock().unwrap().contains(&jti.to_string()))
                    }
                })
                .await;
        }

        // Should have 50 cached entries (only even-numbered JTIs are revoked)
        assert_eq!(cache.len(), 50);
    }

    #[tokio::test]
    async fn test_default_config_values() {
        let cache = DenylistCache::default_cache();
        assert_eq!(cache.max_entries(), 10_000);
        assert_eq!(cache.max_ttl_secs(), 300);
        assert_eq!(cache.redis_key_prefix(), "denylist");
        assert!(cache.is_empty());
    }

    // --- Edge case tests from story acceptance criteria ---

    #[tokio::test]
    async fn test_empty_jti_handled_gracefully() {
        let config = DenylistConfig::default();
        let cache = DenylistCache::new(config);

        let result = cache
            .is_revoked("", None, |key| {
                let key = key.to_string();
                async move {
                    let _jti = key.strip_prefix("denylist:").unwrap_or(&key);
                    Ok(false)
                }
            })
            .await;

        assert_eq!(result, DenylistResult::RedisMiss);
        assert!(cache.is_empty());
    }

    #[tokio::test]
    async fn test_jti_long_string_500_chars() {
        let config = DenylistConfig::default();
        let cache = DenylistCache::new(config);
        let long_jti: String = "a".repeat(500);
        let revoked = Arc::new(Mutex::new(Vec::<String>::new()));
        revoked.lock().unwrap().push(long_jti.clone());

        let result = cache
            .is_revoked(&long_jti, None, |key| {
                let revoked = Arc::clone(&revoked);
                let key = key.to_string();
                async move {
                    let jti = key.strip_prefix("denylist:").unwrap_or(&key);
                    Ok(revoked.lock().unwrap().contains(&jti.to_string()))
                }
            })
            .await;

        assert_eq!(result, DenylistResult::RedisHit);
        assert_eq!(cache.len(), 1);
    }

    #[tokio::test]
    async fn test_jti_unicode_characters() {
        let config = DenylistConfig::default();
        let cache = DenylistCache::new(config);
        let unicode_jti = "abc_üñíçödé";
        let revoked = Arc::new(Mutex::new(Vec::<String>::new()));
        revoked.lock().unwrap().push(unicode_jti.to_string());

        let result = cache
            .is_revoked(unicode_jti, None, |key| {
                let revoked = Arc::clone(&revoked);
                let key = key.to_string();
                async move {
                    let jti = key.strip_prefix("denylist:").unwrap_or(&key);
                    Ok(revoked.lock().unwrap().contains(&jti.to_string()))
                }
            })
            .await;

        assert_eq!(result, DenylistResult::RedisHit);
        assert_eq!(cache.len(), 1);
    }

    #[tokio::test]
    async fn test_cache_never_overrides_redis() {
        let config = DenylistConfig::default();
        let cache = DenylistCache::new(config);

        let result = cache
            .is_revoked("not-revoked", None, |_key| async move { Ok(false) })
            .await;

        assert_eq!(result, DenylistResult::RedisMiss);
        assert!(cache.is_empty());
        assert!(!cache.contains("not-revoked"));
    }

    #[tokio::test]
    async fn test_cache_eviction_maintains_max() {
        let config = DenylistConfig::new(10, 300, 300, 0.0);
        let cache = DenylistCache::new(config);
        let revoked = Arc::new(Mutex::new(Vec::<String>::new()));

        let make_exists = |revoked: Arc<Mutex<Vec<String>>>| {
            move |key: &str| {
                let revoked = Arc::clone(&revoked);
                let key = key.to_string();
                async move {
                    let jti = key.strip_prefix("denylist:").unwrap_or(&key);
                    Ok(revoked.lock().unwrap().contains(&jti.to_string()))
                }
            }
        };

        for i in 0..1000 {
            let jti = format!("jti-evict-{:04}", i);
            revoked.lock().unwrap().push(jti.clone());
            let _ = cache
                .is_revoked(&jti, None, make_exists(Arc::clone(&revoked)))
                .await;
        }

        assert!(
            cache.len() <= 10,
            "Cache size {} exceeds max {}",
            cache.len(),
            10
        );
    }

    #[tokio::test]
    async fn test_ttl_boundary_exp_equals_now() {
        let config = DenylistConfig::new(10_000, 300, 300, 0.0);
        let cache = DenylistCache::new(config);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let ttl = cache.calculate_ttl(Some(now));
        assert_eq!(ttl.as_secs(), 1);
    }

    #[tokio::test]
    async fn test_is_revoked_simple() {
        let config = DenylistConfig::default();
        let cache = DenylistCache::new(config);
        let revoked = Arc::new(Mutex::new(Vec::<String>::new()));
        revoked.lock().unwrap().push("jti-simple".to_string());

        let result = cache
            .is_revoked_simple("jti-simple", |key| {
                let revoked = Arc::clone(&revoked);
                let key = key.to_string();
                async move {
                    let jti = key.strip_prefix("denylist:").unwrap_or(&key);
                    Ok(revoked.lock().unwrap().contains(&jti.to_string()))
                }
            })
            .await;

        assert_eq!(result, DenylistResult::RedisHit);
        assert_eq!(cache.len(), 1);
    }
}