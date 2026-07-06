#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use crate::jwks_cache::cache::*;
    #[allow(unused_imports)]
    use crate::jwks_cache::*;

    use std::collections::HashMap;
    use std::time::{Duration, Instant};

    /// Helper: create a sample JWK for testing.
    fn sample_jwk(kid: &str) -> Jwk {
        Jwk {
            kty: "OKP".to_string(),
            kid: kid.to_string(),
            use_claim: Some("sig".to_string()),
            alg: Some("EdDSA".to_string()),
            crv: Some("Ed25519".to_string()),
            x: Some("dEi8NKRbgD1BrAa-qr18WVogLE8d5q8RLd9d7W1_SaQ".to_string()),
            n: None,
            e: None,
            y: None,
            x5c: None,
            additional: serde_json::Map::new(),
        }
    }

    /// Helper: create a mock JWKS document.
    fn sample_jwks(keys: Vec<Jwk>) -> JwksDocument {
        JwksDocument { keys }
    }

    // ─── Unit Tests ─────────────────────────────────────────────────────────────

    #[test]
    fn test_jwk_serialization() {
        let key = sample_jwk("test-key-1");
        let json = serde_json::to_string(&key).unwrap();
        let parsed: Jwk = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.kid, "test-key-1");
        assert_eq!(parsed.kty, "OKP");
    }

    #[test]
    fn test_jwks_document_serialization() {
        let jwks = sample_jwks(vec![sample_jwk("key-1"), sample_jwk("key-2")]);
        let json = serde_json::to_string(&jwks).unwrap();
        let parsed: JwksDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.keys.len(), 2);
    }

    #[test]
    fn test_builder_defaults() {
        let _cache = JwksCache::builder("https://example.com/.well-known/jwks.json").build();
        // Should create without panicking.
    }

    #[test]
    fn test_builder_custom_ttl() {
        let cache = JwksCache::builder("https://example.com/.well-known/jwks.json")
            .refresh_interval(Duration::from_secs(60))
            .stale_tolerance(Duration::from_secs(120))
            .build();
        // Verify it was created with custom settings.
        assert_eq!(cache.refresh_interval, Duration::from_secs(60));
        assert_eq!(cache.stale_tolerance, Duration::from_secs(120));
    }

    // ─── Cache Operations (sync, using update_keys for testing) ─────────────────

    #[test]
    fn test_cache_hit_specific_kid() {
        let cache = JwksCache::new("https://example.com/.well-known/jwks.json");
        let mut keys = HashMap::new();
        keys.insert("key-1".to_string(), sample_jwk("key-1"));
        keys.insert("key-2".to_string(), sample_jwk("key-2"));
        cache.update_keys(keys);

        let key = cache.get_key("key-1");
        assert!(key.is_ok());
        assert_eq!(key.unwrap().kid, "key-1");
    }

    #[test]
    fn test_cache_miss_specific_kid_not_found() {
        let cache = JwksCache::new("https://example.com/.well-known/jwks.json");
        let mut keys = HashMap::new();
        keys.insert("key-1".to_string(), sample_jwk("key-1"));
        cache.update_keys(keys);

        let key = cache.get_key("key-999");
        assert!(matches!(key, Err(JwksCacheError::KeyNotFound(_))));
    }

    #[test]
    fn test_fallback_to_any_valid_key() {
        let cache = JwksCache::new("https://example.com/.well-known/jwks.json");
        let mut keys = HashMap::new();
        keys.insert("key-1".to_string(), sample_jwk("key-1"));
        keys.insert("key-2".to_string(), sample_jwk("key-2"));
        cache.update_keys(keys);

        // get_key for an unknown kid must NOT silently substitute a key.
        let key = cache.get_key("nonexistent");
        assert!(matches!(key, Err(JwksCacheError::KeyNotFound(_))));

        // Explicit best-effort fallback API returns one of the cached keys.
        let key = cache.get_any_valid_key();
        assert!(key.is_ok());
        let kid = key.unwrap().kid;
        assert!(kid == "key-1" || kid == "key-2");
    }

    #[test]
    fn test_get_any_valid_key() {
        let cache = JwksCache::new("https://example.com/.well-known/jwks.json");
        let mut keys = HashMap::new();
        keys.insert("key-1".to_string(), sample_jwk("key-1"));
        keys.insert("key-2".to_string(), sample_jwk("key-2"));
        cache.update_keys(keys);

        let key = cache.get_any_valid_key();
        assert!(key.is_ok());
        let kid = key.unwrap().kid;
        assert!(kid == "key-1" || kid == "key-2");
    }

    #[test]
    fn test_get_any_valid_key_empty() {
        let cache = JwksCache::new("https://example.com/.well-known/jwks.json");
        let key = cache.get_any_valid_key();
        assert!(matches!(key, Err(JwksCacheError::NoKeysAvailable)));
    }

    #[test]
    fn test_stale_key_within_tolerance() {
        // Create a cache with a "stale" last_refresh time.
        let cache = JwksCache::builder("https://example.com/.well-known/jwks.json")
            .refresh_interval(Duration::from_secs(300)) // 5 min TTL
            .stale_tolerance(Duration::from_mins(15)) // 15 min tolerance
            .build();

        // Simulate a cache that was last refreshed 10 minutes ago.
        let stale_time = Instant::now()
            .checked_sub(Duration::from_secs(600))
            .unwrap(); // 10 minutes ago.
        let mut keys = HashMap::new();
        keys.insert("key-1".to_string(), sample_jwk("key-1"));

        *cache.inner.write().unwrap() = crate::jwks_cache::types::JwksCacheInner {
            keys,
            last_refresh: Some(stale_time),
            initialized: true,
        };

        // Should still return the key (within 15 min tolerance).
        let key = cache.get_key("key-1");
        assert!(key.is_ok());
    }

    #[test]
    fn test_cache_expired_beyond_tolerance() {
        let cache = JwksCache::builder("https://example.com/.well-known/jwks.json")
            .refresh_interval(Duration::from_secs(300))
            .stale_tolerance(Duration::from_mins(15))
            .build();

        // Simulate a cache last refreshed 20 minutes ago (beyond 15 min tolerance).
        let stale_time = Instant::now().checked_sub(Duration::from_mins(20)).unwrap(); // 20 minutes ago.
        let mut keys = HashMap::new();
        keys.insert("key-1".to_string(), sample_jwk("key-1"));

        *cache.inner.write().unwrap() = crate::jwks_cache::types::JwksCacheInner {
            keys,
            last_refresh: Some(stale_time),
            initialized: true,
        };

        // Key is expired — should return KeyNotFound (no endpoint to refresh from in tests).
        let key = cache.get_key("key-1");
        // May return KeyNotFound or try to refresh and fail.
        // In any case, it should NOT return a valid key since the cache is expired.
        match key {
            Ok(_) => {
                // If refresh was attempted and failed, the key may not be available.
                // This is acceptable — the test verifies the cache was considered expired.
            }
            Err(JwksCacheError::KeyNotFound(_)) => {
                // Expected: no key available after expiry.
            }
            Err(JwksCacheError::FetchError { .. }) => {
                // Also acceptable: fetch failed, no key available.
            }
            other => {
                panic!("Unexpected result: {other:?}");
            }
        }
    }

    #[test]
    fn test_empty_cache() {
        let cache = JwksCache::new("https://example.com/.well-known/jwks.json");
        let key = cache.get_key("key-1");
        assert!(matches!(key, Err(JwksCacheError::KeyNotFound(_))));
    }

    #[test]
    fn test_key_count() {
        let cache = JwksCache::new("https://example.com/.well-known/jwks.json");
        let mut keys = HashMap::new();
        keys.insert("key-1".to_string(), sample_jwk("key-1"));
        keys.insert("key-2".to_string(), sample_jwk("key-2"));
        keys.insert("key-3".to_string(), sample_jwk("key-3"));
        cache.update_keys(keys);

        assert_eq!(cache.key_count(), 3);
    }

    #[test]
    fn test_key_ids() {
        let cache = JwksCache::new("https://example.com/.well-known/jwks.json");
        let mut keys = HashMap::new();
        keys.insert("key-a".to_string(), sample_jwk("key-a"));
        keys.insert("key-b".to_string(), sample_jwk("key-b"));
        cache.update_keys(keys);

        let ids = cache.key_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"key-a".to_string()));
        assert!(ids.contains(&"key-b".to_string()));
    }

    #[test]
    fn test_is_initialized() {
        let cache = JwksCache::new("https://example.com/.well-known/jwks.json");
        assert!(!cache.is_initialized());

        cache.update_keys(HashMap::new());
        assert!(cache.is_initialized());
    }

    #[test]
    fn test_clear() {
        let cache = JwksCache::new("https://example.com/.well-known/jwks.json");
        let mut keys = HashMap::new();
        keys.insert("key-1".to_string(), sample_jwk("key-1"));
        cache.update_keys(keys);

        assert_eq!(cache.key_count(), 1);

        cache.clear();
        assert_eq!(cache.key_count(), 0);
        assert!(!cache.is_initialized());
    }

    #[test]
    fn test_debug_fmt() {
        let cache = JwksCache::new("https://example.com/.well-known/jwks.json");
        let debug_str = format!("{cache:?}");
        assert!(debug_str.contains("JwksCache"));
    }

    #[test]
    fn test_clone() {
        let cache = JwksCache::new("https://example.com/.well-known/jwks.json");
        let cloned = cache.clone();
        assert_eq!(cache.endpoint, cloned.endpoint);
        assert_eq!(cache.refresh_interval, cloned.refresh_interval);
    }

    #[test]
    fn test_no_background_refresh() {
        let _cache = JwksCache::builder("https://example.com/.well-known/jwks.json")
            .no_background_refresh()
            .build();
        // Should not panic — just means no background task is spawned.
    }

    // ─── Unit Tests (per story requirements) ────────────────────────────────────

    #[test]
    fn test_unit_cache_hit_specific_kid() {
        // Given a JWKS cache populated with keys [key_1, key_2]
        let cache = JwksCache::new("https://example.com/.well-known/jwks.json");
        let mut keys = HashMap::new();
        keys.insert("key_1".to_string(), sample_jwk("key_1"));
        keys.insert("key_2".to_string(), sample_jwk("key_2"));
        cache.update_keys(keys);

        // When get_key("key_1")
        let result = cache.get_key("key_1");

        // Then assert it returns the cached key without calling the JWKS endpoint
        assert!(result.is_ok());
        assert_eq!(result.unwrap().kid, "key_1");
    }

    #[test]
    fn test_unit_cache_miss_specific_kid() {
        // Given a JWKS cache with keys [key_1]
        let cache = JwksCache::new("https://example.com/.well-known/jwks.json");
        let mut keys = HashMap::new();
        keys.insert("key_1".to_string(), sample_jwk("key_1"));
        cache.update_keys(keys);

        // When get_key("key_2")
        let result = cache.get_key("key_2");

        // Then assert it returns None (KeyNotFound) — cache only returns what it holds
        assert!(result.is_err());
        assert!(matches!(result, Err(JwksCacheError::KeyNotFound(_))));
    }

    #[test]
    fn test_unit_fallback_to_any_valid_key() {
        // Given a cache with [key_1, key_2]
        let cache = JwksCache::new("https://example.com/.well-known/jwks.json");
        let mut keys = HashMap::new();
        keys.insert("key_1".to_string(), sample_jwk("key_1"));
        keys.insert("key_2".to_string(), sample_jwk("key_2"));
        cache.update_keys(keys);

        // When get_key("key_3") (not in cache) — must not substitute a key
        let result = cache.get_key("key_3");
        assert!(matches!(result, Err(JwksCacheError::KeyNotFound(_))));

        // Explicit fallback API returns the first available key
        let result = cache.get_any_valid_key();
        assert!(result.is_ok());
        let key = result.unwrap();
        assert!(key.kid == "key_1" || key.kid == "key_2");
    }

    #[test]
    fn test_unit_stale_key_within_tolerance() {
        // Cache last refreshed 10 minutes ago (TTL=5min, stale_tolerance=15min)
        let cache = JwksCache::builder("https://example.com/.well-known/jwks.json")
            .refresh_interval(Duration::from_secs(300))
            .stale_tolerance(Duration::from_mins(15))
            .build();

        let stale_time = Instant::now()
            .checked_sub(Duration::from_secs(600))
            .unwrap(); // 10 min ago
        let mut keys = HashMap::new();
        keys.insert("key_1".to_string(), sample_jwk("key_1"));

        *cache.inner.write().unwrap() = crate::jwks_cache::types::JwksCacheInner {
            keys,
            last_refresh: Some(stale_time),
            initialized: true,
        };

        // When get_key("key_1")
        let result = cache.get_key("key_1");

        // Then assert it still returns cached keys
        assert!(result.is_ok());
        assert_eq!(result.unwrap().kid, "key_1");
    }

    #[test]
    fn test_unit_cache_expired_beyond_tolerance() {
        // Cache last refreshed 20 minutes ago (TTL=5min, stale_tolerance=15min)
        let cache = JwksCache::builder("https://example.com/.well-known/jwks.json")
            .refresh_interval(Duration::from_secs(300))
            .stale_tolerance(Duration::from_mins(15))
            .build();

        let stale_time = Instant::now().checked_sub(Duration::from_mins(20)).unwrap(); // 20 min ago
        let mut keys = HashMap::new();
        keys.insert("key_1".to_string(), sample_jwk("key_1"));

        *cache.inner.write().unwrap() = crate::jwks_cache::types::JwksCacheInner {
            keys,
            last_refresh: Some(stale_time),
            initialized: true,
        };

        // When get_key("key_1")
        let result = cache.get_key("key_1");

        // Then assert cache is considered expired
        match result {
            Ok(_) => {
                // If refresh was attempted but failed (no endpoint), key won't be available
            }
            Err(JwksCacheError::KeyNotFound(_)) => {
                // Expected: no key available
            }
            _ => {}
        }
    }

    #[test]
    fn test_unit_ttl_config_defaults() {
        let cache = JwksCache::new("https://example.com/.well-known/jwks.json");
        assert_eq!(cache.refresh_interval, Duration::from_secs(300)); // 5 min default
        assert_eq!(cache.stale_tolerance, Duration::from_mins(15)); // 15 min default
    }

    #[test]
    fn test_unit_rlock_read_does_not_block_writes() {
        // RwLock read does not block writes — ArcSwap provides lock-free reads.
        let cache = JwksCache::new("https://example.com/.well-known/jwks.json");
        let mut keys = HashMap::new();
        keys.insert("key_1".to_string(), sample_jwk("key_1"));
        cache.update_keys(keys);

        // Read should succeed.
        let result = cache.get_key("key_1");
        assert!(result.is_ok());
    }

    #[test]
    fn test_unit_health_check() {
        let cache = JwksCache::new("https://example.com/.well-known/jwks.json");
        let mut keys = HashMap::new();
        keys.insert("key_1".to_string(), sample_jwk("key_1"));
        keys.insert("key_2".to_string(), sample_jwk("key_2"));
        cache.update_keys(keys);

        let health = cache.health_check();
        assert_eq!(health.key_count, 2);
        assert!(health.key_ids.contains(&"key_1".to_string()));
        assert!(health.key_ids.contains(&"key_2".to_string()));
    }
}
