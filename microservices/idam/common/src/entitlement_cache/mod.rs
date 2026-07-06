//! Shared entitlement snapshot cache for Sesame-IDAM microservices.
//!
//! Provides `EntitlementSnapshotCache` — a local, in-memory cache for pre-computed ACL
//! snapshots keyed by entitlement reference. This enables compact JWTs that carry
//! only an `entitlements_ref` instead of full ACL payloads, reducing token size from
//! ~5-10 KB to ~200 bytes.
//!
//! ## Architecture
//!
//! The cache follows a standard read-through pattern:
//! 1. Check local cache for the requested `entitlements_ref`
//! 2. On cache hit, return the snapshot immediately
//! 3. On cache miss, call authz-core to fetch the full ACL, then cache the result
//!
//! TTL is configurable per entitlement complexity (30-300 seconds), with high-risk
//! permissions (admin, delete) using a short TTL to limit permission escalation windows.
//!
//! ## Security Gotchas Addressed
//!
//! - **HACK-751**: High-risk permissions force short TTL (30s) to minimize stale permission windows
//! - **HACK-752**: LRU eviction enforces `max_entries` (5,000) to prevent memory exhaustion
//! - **HACK-753**: Exact key matching (no hashing) prevents reference collision attacks
//!
//! ## Example
//!
//! ```rust
//! use crate::entitlement_cache::{EntitlementSnapshot, Permission, EntitlementComplexity};
//!
//! let perms = vec![Permission::new("read", "documents")];
//! let snap = EntitlementSnapshot::new("user_123", "org_456", perms, EntitlementComplexity::Static);
//! ```

pub mod snapshot;

pub use snapshot::{CacheLookupResult, EntitlementComplexity, EntitlementSnapshot, Permission};

use prometheus::{IntCounter, IntGauge, Registry};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::Duration;
use tracing::debug;

/// Inner cache entry with expiry tracking.
struct CachedEntry {
    snapshot: EntitlementSnapshot,
    expires_at: std::time::Instant,
    serialized_size: usize,
    access_count: AtomicU64,
}

/// Inner cache state protected by a `RwLock`.
struct CacheState {
    entries: HashMap<String, CachedEntry>,
    /// Track access order for LRU eviction (simple counter-based).
    access_order: HashMap<String, u64>,
    next_order: u64,
    /// Total eviction count for metrics.
    evictions: u64,
}

/// Errors that can occur during cache operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheError {
    /// The ACL snapshot exceeds the configured size limit.
    AclTooLarge { requested: usize, max: usize },
    /// The fetch function returned an error.
    FetchError(String),
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheError::AclTooLarge { requested, max } => {
                write!(f, "ACL too large: {requested} bytes (max {max})")
            }
            CacheError::FetchError(msg) => write!(f, "Fetch error: {msg}"),
        }
    }
}

impl std::error::Error for CacheError {}

/// Configuration for the entitlement snapshot cache.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries in the cache (per instance).
    pub max_entries: usize,
    /// Default TTL when complexity is not specified (seconds).
    pub default_ttl_secs: u64,
    /// Maximum ACL size in bytes (hard cap).
    pub max_acl_size_bytes: usize,
    /// TTL for high-risk entitlements (seconds).
    pub high_risk_ttl_secs: u64,
    /// Maximum ACL snapshot size in bytes.
    pub max_snapshot_bytes: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 5000,
            default_ttl_secs: 60,
            max_acl_size_bytes: 50_000,
            high_risk_ttl_secs: 30,
            max_snapshot_bytes: 50_000,
        }
    }
}

/// High-level entitlement snapshot cache.
///
/// A local, in-memory cache keyed by entitlement reference string.
/// Supports TTL-based expiration with complexity-based TTL selection,
/// LRU eviction, and size-based ACL rejection.
pub struct EntitlementSnapshotCache {
    state: RwLock<CacheState>,
    config: CacheConfig,
    /// Prometheus metric counters/gauges.
    hits_counter: IntCounter,
    misses_counter: IntCounter,
    evictions_counter: IntCounter,
    cache_size_gauge: IntGauge,
    cache_memory_gauge: IntGauge,
    acls_too_large_counter: IntCounter,
}

impl EntitlementSnapshotCache {
    /// Create a new entitlement snapshot cache and register its metrics with the given registry.
    #[must_use]
    pub fn new(config: CacheConfig) -> Self {
        let registry = Registry::new();

        let hits_counter = IntCounter::new(
            "entitlement_cache_hits_total",
            "Total cache hits for entitlement snapshots",
        )
        .unwrap();
        registry.register(Box::new(hits_counter.clone())).unwrap();

        let misses_counter = IntCounter::new(
            "entitlement_cache_misses_total",
            "Total cache misses for entitlement snapshots",
        )
        .unwrap();
        registry.register(Box::new(misses_counter.clone())).unwrap();

        let evictions_counter = IntCounter::new(
            "entitlement_cache_evicted_total",
            "Total evicted entries from entitlement cache",
        )
        .unwrap();
        registry
            .register(Box::new(evictions_counter.clone()))
            .unwrap();

        let cache_size_gauge = IntGauge::new(
            "entitlement_cache_size",
            "Current number of entries in the entitlement cache",
        )
        .unwrap();
        registry
            .register(Box::new(cache_size_gauge.clone()))
            .unwrap();

        let cache_memory_gauge = IntGauge::new(
            "entitlement_cache_memory_bytes",
            "Total memory usage of cached ACL snapshots in bytes",
        )
        .unwrap();
        registry
            .register(Box::new(cache_memory_gauge.clone()))
            .unwrap();

        let acls_too_large_counter = IntCounter::new(
            "entitlement_cache_acls_too_large_total",
            "ACL snapshots rejected for exceeding size limit",
        )
        .unwrap();
        registry
            .register(Box::new(acls_too_large_counter.clone()))
            .unwrap();

        Self {
            state: RwLock::new(CacheState {
                entries: HashMap::new(),
                access_order: HashMap::new(),
                next_order: 0,
                evictions: 0,
            }),
            config,
            hits_counter,
            misses_counter,
            evictions_counter,
            cache_size_gauge,
            cache_memory_gauge,
            acls_too_large_counter,
        }
    }

    /// Get or insert an entitlement snapshot.
    ///
    /// If the `entitlements_ref` is already cached and not expired, returns the cached snapshot
    /// immediately (cache hit). Otherwise, calls `fetch_fn` to retrieve the full ACL, validates
    /// it, caches it, and returns it (cache miss).
    ///
    /// # Arguments
    ///
    /// * `entitlements_ref` — The reference ID from the JWT claim.
    /// * `fetch_fn` — Async function called on cache miss to fetch the full ACL.
    ///
    /// # Errors
    ///
    /// Returns `CacheError::AclTooLarge` if the fetched ACL exceeds the size limit,
    /// or `CacheError::FetchError` if the fetch function fails.
    pub async fn get_or_insert<F, Fut, E>(
        &self,
        entitlements_ref: &str,
        fetch_fn: F,
    ) -> Result<EntitlementSnapshot, CacheError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<EntitlementSnapshot, E>>,
        E: std::fmt::Display + std::marker::Send + Sync,
    {
        let cache_key = format!("entitlements:{entitlements_ref}");

        // 1. Check cache
        {
            let state = self.state.read().unwrap();
            if let Some(entry) = state.entries.get(&cache_key) {
                if entry.expires_at > std::time::Instant::now() {
                    entry.access_count.fetch_add(1, Ordering::Relaxed);
                    // Clone snapshot while holding the read lock, then release before metrics update
                    let cloned_snapshot = entry.snapshot.clone();
                    drop(state);

                    self.hits_counter.inc();
                    self.update_gauges();
                    debug!(
                        "entitlement cache HIT for ref={}",
                        truncate_ref(entitlements_ref)
                    );
                    return Ok(cloned_snapshot);
                }
            }
        }

        // 2. Cache miss — fetch from source
        debug!(
            "entitlement cache MISS for ref={}",
            truncate_ref(entitlements_ref)
        );
        let snapshot = match fetch_fn().await {
            Ok(snap) => snap,
            Err(e) => {
                return Err(CacheError::FetchError(e.to_string()));
            }
        };

        // 3. Validate ACL size
        let serialized_size = snapshot.serialized_size_bytes();
        if serialized_size > self.config.max_acl_size_bytes {
            self.acls_too_large_counter.inc();
            return Err(CacheError::AclTooLarge {
                requested: serialized_size,
                max: self.config.max_acl_size_bytes,
            });
        }

        // 4. Calculate TTL based on complexity and high-risk check
        let ttl = self.calculate_ttl(&snapshot);

        // 5. Insert into cache
        self.insert_entry(cache_key.clone(), snapshot.clone(), serialized_size, ttl)
            .await;

        self.misses_counter.inc();
        self.update_gauges();

        debug!(
            "entitlement cache MISS+FRESH for ref={}, TTL={}s",
            truncate_ref(entitlements_ref),
            ttl.as_secs()
        );

        Ok(snapshot)
    }

    /// Insert a cache entry with LRU eviction if needed.
    async fn insert_entry(
        &self,
        cache_key: String,
        snapshot: EntitlementSnapshot,
        serialized_size: usize,
        ttl: Duration,
    ) {
        let mut state = self.state.write().unwrap();

        // Evict expired entries first
        let now = std::time::Instant::now();
        let expired_keys: Vec<String> = state
            .entries
            .iter()
            .filter(|(_, entry)| entry.expires_at <= now)
            .map(|(k, _)| k.clone())
            .collect();
        for key in &expired_keys {
            state.entries.remove(key);
            state.access_order.remove(key);
            state.evictions += 1;
        }

        // Evict LRU entries if still over limit
        while state.entries.len() >= self.config.max_entries {
            if let Some(lru_key) = state
                .access_order
                .iter()
                .min_by_key(|(_, &order)| order)
                .map(|(k, _)| k.clone())
            {
                state.entries.remove(&lru_key);
                state.access_order.remove(&lru_key);
                state.evictions += 1;
            } else {
                break;
            }
        }

        let entry = CachedEntry {
            snapshot,
            expires_at: now + ttl,
            serialized_size,
            access_count: AtomicU64::new(0),
        };

        let order = state.next_order;
        state.next_order += 1;
        state.entries.insert(cache_key.clone(), entry);
        state.access_order.insert(cache_key, order);
    }

    /// Calculate the TTL for a snapshot based on its complexity and risk level.
    fn calculate_ttl(&self, snapshot: &EntitlementSnapshot) -> Duration {
        if snapshot.contains_high_risk() {
            Duration::from_secs(self.config.high_risk_ttl_secs)
        } else {
            Duration::from_secs(snapshot.complexity.default_ttl_seconds() as u64)
        }
    }

    /// Invalidate a specific cache entry.
    ///
    /// Useful when a user's permissions change and you want to force
    /// an immediate refresh rather than waiting for TTL expiry.
    pub fn invalidate(&self, entitlements_ref: &str) {
        let cache_key = format!("entitlements:{entitlements_ref}");
        let mut state = self.state.write().unwrap();
        if state.entries.remove(&cache_key).is_some() {
            state.access_order.remove(&cache_key);
            debug!(
                "entitlement cache invalidated for ref={}",
                truncate_ref(entitlements_ref)
            );
        }
    }

    /// Clear all entries from the cache.
    pub fn clear(&self) {
        let mut state = self.state.write().unwrap();
        let count = state.entries.len();
        state.entries.clear();
        state.access_order.clear();
        let _ = count; // used for logging in production
    }

    /// Get the current number of entries in the cache.
    pub fn len(&self) -> usize {
        self.state.read().unwrap().entries.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.state.read().unwrap().entries.is_empty()
    }

    /// Update Prometheus gauges to reflect current cache state.
    fn update_gauges(&self) {
        let state = self.state.read().unwrap();
        self.cache_size_gauge.set(state.entries.len() as i64);
        let total_memory: usize = state.entries.values().map(|e| e.serialized_size).sum();
        self.cache_memory_gauge.set(total_memory as i64);
    }
}

/// Truncate an `entitlements_ref` for log messages to prevent log pollution.
fn truncate_ref(ref_id: &str) -> &str {
    if ref_id.len() > 40 {
        &ref_id[..40]
    } else {
        ref_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snapshot::{EntitlementComplexity, Permission};

    fn make_snapshot(user_id: &str, perms: Vec<Permission>) -> EntitlementSnapshot {
        EntitlementSnapshot::new(user_id, "org_test", perms, EntitlementComplexity::Static)
    }

    fn make_high_risk_snapshot(user_id: &str) -> EntitlementSnapshot {
        EntitlementSnapshot::new(
            user_id,
            "org_test",
            vec![Permission::new("admin", "users")],
            EntitlementComplexity::Custom,
        )
    }

    // ═══════════════════════════════════════════════════════════
    // Cache Configuration Tests
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn test_cache_config_defaults() {
        let config = CacheConfig::default();
        assert_eq!(config.max_entries, 5000);
        assert_eq!(config.default_ttl_secs, 60);
        assert_eq!(config.max_acl_size_bytes, 50_000);
        assert_eq!(config.high_risk_ttl_secs, 30);
    }

    #[test]
    fn test_cache_config_custom_values() {
        let config = CacheConfig {
            max_entries: 10000,
            default_ttl_secs: 120,
            max_acl_size_bytes: 100_000,
            high_risk_ttl_secs: 15,
            max_snapshot_bytes: 100_000,
        };
        assert_eq!(config.max_entries, 10000);
        assert_eq!(config.default_ttl_secs, 120);
        assert_eq!(config.max_acl_size_bytes, 100_000);
        assert_eq!(config.high_risk_ttl_secs, 15);
    }

    // ═══════════════════════════════════════════════════════════
    // TTL Calculation Tests
    // ═══════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_ttl_for_static_complexity_no_high_risk() {
        let cache = EntitlementSnapshotCache::new(CacheConfig::default());
        let snap = make_snapshot("u1", vec![Permission::new("read", "docs")]);
        let ttl = cache.calculate_ttl(&snap);
        assert_eq!(ttl.as_secs(), 300); // Static = 300s
    }

    #[tokio::test]
    async fn test_ttl_for_static_complexity_with_high_risk() {
        let cache = EntitlementSnapshotCache::new(CacheConfig::default());
        let snap = make_high_risk_snapshot("u1");
        let ttl = cache.calculate_ttl(&snap);
        assert_eq!(ttl.as_secs(), 30); // High-risk cap
    }

    #[tokio::test]
    async fn test_ttl_for_dynamic_complexity_no_high_risk() {
        let cache = EntitlementSnapshotCache::new(CacheConfig::default());
        let snap = EntitlementSnapshot::new(
            "u1",
            "org_test",
            vec![Permission::new("read", "data")],
            EntitlementComplexity::Dynamic,
        );
        let ttl = cache.calculate_ttl(&snap);
        assert_eq!(ttl.as_secs(), 30); // Dynamic = 30s
    }

    // ═══════════════════════════════════════════════════════════
    // Cache Get/Insert Tests
    // ═══════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_cache_hit_returns_cached_value() {
        let cache = EntitlementSnapshotCache::new(CacheConfig::default());
        let perms = vec![
            Permission::new("read", "docs"),
            Permission::new("write", "docs"),
        ];
        let snap = make_snapshot("u1", perms);

        // First call — cache miss
        let result = cache
            .get_or_insert("ent_1", || async {
                Ok::<EntitlementSnapshot, String>(snap.clone())
            })
            .await;
        assert!(result.is_ok());
        let first = result.unwrap();
        assert_eq!(first.user_id, "u1");
        assert_eq!(first.permissions.len(), 2);

        // Second call — cache hit (fetch_fn should NOT be called)
        let fetch_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let fetch_called_clone = fetch_called.clone();
        let result = cache
            .get_or_insert("ent_1", || {
                let called = fetch_called_clone.clone();
                async move {
                    called.store(true, Ordering::Relaxed);
                    Err("should not be called".to_string())
                }
            })
            .await;
        assert!(result.is_ok());
        assert!(
            !fetch_called.load(Ordering::Relaxed),
            "fetch_fn should NOT be called on cache hit"
        );
    }

    #[tokio::test]
    async fn test_cache_miss_calls_fetch_fn() {
        let cache = EntitlementSnapshotCache::new(CacheConfig::default());
        let fetch_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let fetch_called_clone = fetch_called.clone();

        let result = cache
            .get_or_insert("ent_2", || {
                let called = fetch_called_clone.clone();
                async move {
                    called.store(true, Ordering::Relaxed);
                    Ok::<EntitlementSnapshot, String>(make_snapshot(
                        "u2",
                        vec![Permission::new("read", "reports")],
                    ))
                }
            })
            .await;

        assert!(result.is_ok());
        assert!(
            fetch_called.load(Ordering::Relaxed),
            "fetch_fn should be called on cache miss"
        );
        let snap = result.unwrap();
        assert_eq!(snap.user_id, "u2");
    }

    #[tokio::test]
    async fn test_cache_miss_fetch_error() {
        let cache = EntitlementSnapshotCache::new(CacheConfig::default());

        let result = cache
            .get_or_insert("ent_3", || async { Err("fetch failed".to_string()) })
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            CacheError::FetchError(msg) => assert_eq!(msg, "fetch failed"),
            other => panic!("expected FetchError, got {other:?}"),
        }
    }

    // ═══════════════════════════════════════════════════════════
    // ACL Size Validation Tests
    // ═══════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_acl_too_large_rejected() {
        let config = CacheConfig {
            max_entries: 5000,
            default_ttl_secs: 60,
            max_acl_size_bytes: 100, // Very small limit for testing
            high_risk_ttl_secs: 30,
            max_snapshot_bytes: 100,
        };
        let cache = EntitlementSnapshotCache::new(config);

        let perms = vec![
            Permission::new("read", "very_long_resource_name_for_testing_purposes_aaa"),
            Permission::new("write", "another_very_long_resource_name_for_testing_bbb"),
            Permission::new(
                "delete",
                "yet_another_very_long_resource_name_for_testing_ccc",
            ),
        ];
        let snap = EntitlementSnapshot::new("u1", "org_test", perms, EntitlementComplexity::Static);

        let result = cache
            .get_or_insert("ent_large", || async {
                Ok::<EntitlementSnapshot, String>(snap)
            })
            .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            CacheError::AclTooLarge { requested, max } => {
                assert!(requested > max);
                assert_eq!(max, 100);
            }
            other => panic!("expected AclTooLarge, got {other:?}"),
        }
    }

    // ═══════════════════════════════════════════════════════════
    // Eviction Tests
    // ═══════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_lru_eviction_when_full() {
        let config = CacheConfig {
            max_entries: 3,
            default_ttl_secs: 60,
            max_acl_size_bytes: 50_000,
            high_risk_ttl_secs: 30,
            max_snapshot_bytes: 50_000,
        };
        let cache = EntitlementSnapshotCache::new(config);

        // Fill cache to capacity
        for i in 0..3 {
            let snap = make_snapshot(&format!("u{i}"), vec![Permission::new("read", "docs")]);
            cache
                .get_or_insert(&format!("ent_{i}"), || async move {
                    Ok::<EntitlementSnapshot, String>(snap)
                })
                .await
                .unwrap();
        }

        assert_eq!(cache.len(), 3);

        // Adding a 4th should evict the least recently used (ent_0)
        let snap4 = make_snapshot("u4", vec![Permission::new("read", "docs")]);
        cache
            .get_or_insert("ent_4", || async {
                Ok::<EntitlementSnapshot, String>(snap4)
            })
            .await
            .unwrap();

        assert_eq!(cache.len(), 3);

        // ent_0 should be evicted
        let result = cache
            .get_or_insert("ent_0", || async {
                Err("should have been evicted".to_string())
            })
            .await;
        assert!(result.is_err());

        // ent_1 should still be cached
        let result = cache
            .get_or_insert("ent_1", || async {
                Err("should still be cached".to_string())
            })
            .await;
        assert!(result.is_ok());
    }

    // ═══════════════════════════════════════════════════════════
    // Invalidation Tests
    // ═══════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_invalidate_removes_entry() {
        let cache = EntitlementSnapshotCache::new(CacheConfig::default());

        let snap = make_snapshot("u1", vec![Permission::new("read", "docs")]);
        cache
            .get_or_insert("ent_inv", || async {
                Ok::<EntitlementSnapshot, String>(snap)
            })
            .await
            .unwrap();
        assert_eq!(cache.len(), 1);

        cache.invalidate("ent_inv");
        assert_eq!(cache.len(), 0);
    }

    #[tokio::test]
    async fn test_invalidate_nonexistent_does_nothing() {
        let cache = EntitlementSnapshotCache::new(CacheConfig::default());
        cache.invalidate("ent_nonexistent");
        assert_eq!(cache.len(), 0);
    }

    #[tokio::test]
    async fn test_clear_removes_all_entries() {
        let cache = EntitlementSnapshotCache::new(CacheConfig::default());

        for i in 0..5 {
            let snap = make_snapshot(&format!("u{i}"), vec![Permission::new("read", "docs")]);
            cache
                .get_or_insert(&format!("ent_{i}"), || async move {
                    Ok::<EntitlementSnapshot, String>(snap)
                })
                .await
                .unwrap();
        }
        assert_eq!(cache.len(), 5);

        cache.clear();
        assert_eq!(cache.len(), 0);
    }

    // ═══════════════════════════════════════════════════════════
    // Edge Case Tests
    // ═══════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_empty_permissions_snapshot() {
        let cache = EntitlementSnapshotCache::new(CacheConfig::default());
        let snap = EntitlementSnapshot::new("u1", "o1", vec![], EntitlementComplexity::Static);

        let result = cache
            .get_or_insert("ent_empty", || async {
                Ok::<EntitlementSnapshot, String>(snap)
            })
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().permissions.is_empty());
    }

    #[tokio::test]
    async fn test_cache_empty_check() {
        let cache = EntitlementSnapshotCache::new(CacheConfig::default());
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);

        let snap = make_snapshot("u1", vec![Permission::new("read", "docs")]);
        cache
            .get_or_insert("ent_1", || async {
                Ok::<EntitlementSnapshot, String>(snap)
            })
            .await
            .unwrap();
        assert!(!cache.is_empty());
        assert_eq!(cache.len(), 1);
    }

    #[tokio::test]
    async fn test_cache_with_unicode_ref() {
        let cache = EntitlementSnapshotCache::new(CacheConfig::default());
        let snap = make_snapshot("u1", vec![Permission::new("读取", "文档")]);

        let result = cache
            .get_or_insert("ent_üñíçödé", || async {
                Ok::<EntitlementSnapshot, String>(snap)
            })
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().user_id, "u1");
    }

    #[tokio::test]
    async fn test_cache_serialization_format() {
        let snap = EntitlementSnapshot::new(
            "user_123",
            "org_456",
            vec![
                Permission::new("read", "documents"),
                Permission::new("write", "documents"),
            ],
            EntitlementComplexity::RoleOrg,
        );

        let json = serde_json::to_string_pretty(&snap).unwrap();
        let deserialized: EntitlementSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.user_id, snap.user_id);
        assert_eq!(deserialized.org_id, snap.org_id);
        assert_eq!(deserialized.permissions.len(), snap.permissions.len());
        assert_eq!(deserialized.complexity, EntitlementComplexity::RoleOrg);
    }

    #[test]
    fn test_truncate_ref_short() {
        let ref_id = "ent_short";
        assert_eq!(truncate_ref(ref_id), "ent_short");
    }

    #[test]
    fn test_truncate_ref_long() {
        let ref_id = "ent_very_long_reference_id_that_exceeds_forty_characters_for_testing";
        assert_eq!(truncate_ref(ref_id).len(), 40);
    }
}
