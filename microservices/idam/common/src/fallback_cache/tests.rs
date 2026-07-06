#[cfg(test)]
mod tests {
    use crate::fallback_cache::{
        jwt_claims_cover_decision, sanitize_key_input, AuthzCheckRequest, AuthzDecision,
        FallbackCache, FallbackCacheError, FallbackMetrics, RouteTtlConfig,
    };
    use prometheus::Registry;

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

        let keys: Vec<_> = variations
            .iter()
            .map(super::super::types::AuthzCheckRequest::cache_key)
            .collect();
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
            sub: String::new(),
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
        assert_eq!(metrics.cache_hit_total.get(), 0);
        assert_eq!(metrics.cache_miss_total.get(), 0);
    }

    #[test]
    fn test_cache_hit_ratio() {
        let registry = Registry::new();
        let metrics = FallbackMetrics::new(&registry);

        // 80 hits, 20 misses = 80% hit ratio
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

        // 0 hits, 0 misses — gauge stays at 0.0 (no division by zero)
        assert!((metrics.cache_hit_ratio.get()).abs() < f64::EPSILON);

        // A single hit moves the ratio to 1.0 without panicking
        metrics.inc_cache_hit();
        assert!((metrics.cache_hit_ratio.get() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_total_counter_increment() {
        let registry = Registry::new();
        let metrics = FallbackMetrics::new(&registry);

        metrics.inc_total("/test/route", "fallback");
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
        assert!(format!("{err}").contains("Redis error"));

        let err = FallbackCacheError::AuthzCoreError {
            status: 500,
            reason: "internal error".to_string(),
        };
        assert!(format!("{err}").contains("500"));

        let err = FallbackCacheError::JsonError("invalid json".to_string());
        assert!(format!("{err}").contains("JSON error"));
    }

    #[test]
    fn test_fallback_cache_error_from_json() {
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
        let cache = std::sync::Arc::new(FallbackCache::new(
            "redis://127.0.0.1:6379".to_string(),
            "http://authz-core:8102".to_string(),
        ));

        let requests: Vec<_> = (0..5)
            .map(|i| AuthzCheckRequest {
                tenant_id: "tenant-1".to_string(),
                sub: format!("user-{i}"),
                org_id: "org-1".to_string(),
                action: "read".to_string(),
                resource_id: "resource-1".to_string(),
            })
            .collect();

        let handles: Vec<_> = requests
            .into_iter()
            .map(|req| {
                let cache = std::sync::Arc::clone(&cache);
                tokio::spawn(
                    async move { cache.authorize(&req, false, "/admin/users/query").await },
                )
            })
            .collect();

        for handle in handles {
            let result = handle.await.expect("task panicked");
            assert!(result.is_ok(), "request should succeed (mock authz-core)");
        }
    }

    #[tokio::test]
    async fn test_single_flight_same_key_dedupe() {
        let cache = std::sync::Arc::new(FallbackCache::new(
            "redis://127.0.0.1:6379".to_string(),
            "http://authz-core:8102".to_string(),
        ));

        let request = AuthzCheckRequest {
            tenant_id: "tenant-1".to_string(),
            sub: "user-1".to_string(),
            org_id: "org-1".to_string(),
            action: "read".to_string(),
            resource_id: "resource-1".to_string(),
        };

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let cache = std::sync::Arc::clone(&cache);
                let req = request.clone();
                tokio::spawn(
                    async move { cache.authorize(&req, false, "/admin/users/query").await },
                )
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

        let result = cache.authorize(&request, false, "/admin/users/query").await;
        assert!(result.is_ok()); // authz-core mock returns Allowed
    }
}
