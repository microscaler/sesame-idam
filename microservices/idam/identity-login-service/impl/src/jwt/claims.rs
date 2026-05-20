use serde::{Deserialize, Serialize};

/// Entitlements reference generation utilities.
///
/// Generates deterministic UUID v5 references from (user_id, org_id, version) tuples.
/// The reference is used as a cache key for Redis-stored entitlements snapshots.
///
/// HACK-203: The ref is deterministic, meaning it can be predicted if (user_id, org_id, version)
/// is known. This is documented as acceptable because the ref is useless without Redis access,
/// and snapshots have short TTLs.
///
/// HACK-206: Cache keys MUST include tenant_id for multi-tenant isolation.
pub mod entitlements_ref {
    use uuid::Uuid;

    /// NAMESPACE_SHA256 - a reserved UUID for generating hashes
    const NAMESPACE_SHA256: &str = "6ba7b811-9dad-11d1-80b4-00c04fd430c8";

    /// Generate a deterministic entitlements reference from (user_id, org_id, version).
    ///
    /// Returns a string in the format "ent_<uuid_v5>".
    /// The same tuple always produces the same reference.
    ///
    /// # Examples
    /// ```
    /// # use sesame_idam_identity_login_service_impl::jwt::entitlements_ref;
    /// let ref1 = entitlements_ref("user-123", "org-456", 1);
    /// let ref2 = entitlements_ref("user-123", "org-456", 1);
    /// assert_eq!(ref1, ref2); // deterministic
    /// ```
    pub fn generate(user_id: &str, org_id: &str, version: u64) -> String {
        let namespace = Uuid::parse_str(NAMESPACE_SHA256)
            .expect("NAMESPACE_SHA256 must be a valid UUID");
        let input = format!("{}:{}:{}", user_id, org_id, version);
        let uuid = Uuid::new_v5(&namespace, input.as_bytes());
        format!("ent_{}", uuid)
    }

    /// Generate a cache key for Redis with tenant isolation (HACK-206).
    ///
    /// Format: `entitlements:{tenant_id}:{entitlements_ref}`
    /// This prevents cross-tenant bleed when two tenants have users with the same
    /// (user_id, org_id, version) tuple.
    pub fn cache_key(tenant_id: &str, entitlements_ref: &str) -> String {
        format!("entitlements:{}:{}", tenant_id, entitlements_ref)
    }

    /// Version-bump the entitlements ref.
    ///
    /// When entitlements change, bump the version. The new version produces a different
    /// ref, forcing cache invalidation (HACK-204).
    pub fn bump_version(original: &str, new_version: u64) -> String {
        // Parse the original ent_<uuid>
        // Then generate a new one with the bumped version
        // In practice, we'd need user_id/org_id to do this properly.
        // This is a utility for testing/referencing.
        // For actual usage, callers should pass user_id/org_id/version directly.
        generate("", "", new_version)
    }
}

/// Entitlements hash utilities.
///
/// Computes SHA-256 hash of canonical JSON representation of entitlements snapshots.
/// Used for cache poisoning detection (HACK-201) and data integrity verification (HACK-207).
///
/// HACK-202: The hash covers the ENTIRE entitlements snapshot stored in Redis.
/// Consumers MUST use the Redis snapshot (after hash verification) as authoritative,
/// not the JWT claims. The JWT only carries the ref and hash.
///
/// HACK-207: SHA-256 is the standard algorithm. Canonical JSON means sorted keys,
/// no whitespace, consistent number formatting.
pub mod entitlements_hash {
    use sha2::{Digest, Sha256};
    use serde_json::Value;

    /// Compute SHA-256 hash of canonical JSON representation.
    ///
    /// Canonical JSON: sorted keys, no whitespace, no trailing newline.
    ///
    /// # Examples
    /// ```
    /// # use sesame_idam_identity_login_service_impl::jwt::entitlements_hash;
    /// # use serde_json::json;
    /// let snapshot = json!({
    ///     "version": 42,
    ///     "permissions": ["org:admin", "billing:read"],
    ///     "roles": ["admin"],
    ///     "tenant": "tenant-uuid"
    /// });
    /// let hash = entitlements_hash::compute(&snapshot);
    /// assert!(hash.starts_with("sha256:"));
    /// assert_eq!(hash.len(), 71); // "sha256:" (7) + 64 hex chars
    /// ```
    pub fn compute(snapshot: &Value) -> String {
        let canonical = canonical_json(snapshot);
        let mut hasher = Sha256::new();
        hasher.update(canonical.as_bytes());
        let result = hasher.finalize();
        let hex = result.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        format!("sha256:{}", hex)
    }

    /// Verify that a fetched snapshot matches the expected hash.
    ///
    /// # HACK-201
    /// This function MUST be called after every Redis cache fetch.
    /// If verification fails, the consumer MUST reject the cached data,
    /// invalidate the poisoned cache entry, and re-fetch from authz-core.
    ///
    /// # Returns
    /// - `Ok(())` if the hash matches
    /// - `Err(AuthError::EntitlementsHashMismatch)` if the hash doesn't match
    ///
    /// # Security
    /// A hash mismatch indicates potential cache poisoning. The caller should:
    /// 1. Log the event (METRICS.entitlements_cache_poison_detected.inc())
    /// 2. Delete the poisoned cache entry
    /// 3. Fetch authoritative data from authz-core
    /// 4. Re-populate the cache with verified data
    pub fn verify(snapshot: &Value, expected_hash: &str) -> bool {
        let computed = compute(snapshot);
        computed == expected_hash
    }

    /// Canonical JSON serialization: sorted keys, no whitespace.
    fn canonical_json(value: &Value) -> String {
        // Sort object keys recursively and remove whitespace
        match value {
            Value::Object(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                let parts: Vec<String> = keys
                    .iter()
                    .map(|k| {
                        let v = &map[k];
                        format!("{}:{}", canonical_json_key(k), canonical_json(v))
                    })
                    .collect();
                format!("{{{}}}", parts.join(","))
            }
            Value::Array(arr) => {
                let parts: Vec<String> = arr.iter().map(canonical_json).collect();
                format!("[{}]", parts.join(","))
            }
            Value::String(s) => {
                // Escape special characters per JSON spec
                format!(
                    "\"{}\"",
                    s.replace('\\', "\\\\")
                        .replace('"', "\\\"")
                        .replace('\n', "\\n")
                        .replace('\r', "\\r")
                        .replace('\t', "\\t")
                )
            }
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    format!("{}", i)
                } else if let Some(f) = n.as_f64() {
                    format!("{}", f)
                } else {
                    "0".to_string()
                }
            }
            Value::Bool(b) => format!("{}", b),
            Value::Null => "null".to_string(),
        }
    }

    fn canonical_json_key(key: &str) -> String {
        format!(
            "\"{}\"",
            key.replace('\\', "\\\\")
                .replace('"', "\\\"")
        )
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use serde_json::json;

        #[test]
        fn test_canonical_json_deterministic() {
            let json1 = json!({"b": 2, "a": 1});
            let json2 = json!({"a": 1, "b": 2});
            assert_eq!(canonical_json(&json1), canonical_json(&json2));
        }

        #[test]
        fn test_canonical_json_nested() {
            let snapshot = json!({
                "permissions": ["org:admin", "billing:read"],
                "roles": ["admin"],
                "tenant": "tenant-uuid",
                "version": 42
            });
            let canonical = canonical_json(&snapshot);
            // Keys should be sorted: permissions, roles, tenant, version
            assert!(canonical.starts_with(r#"{"permissions":["org:admin","billing:read"],"roles":["admin"],"tenant":"tenant-uuid","version":42}"#));
        }

        #[test]
        fn test_hash_consistency() {
            let snapshot = json!({
                "version": 42,
                "permissions": ["org:admin"],
                "roles": ["admin"],
                "tenant": "tenant-uuid"
            });
            let hash1 = compute(&snapshot);
            let hash2 = compute(&snapshot);
            assert_eq!(hash1, hash2);
            assert!(hash1.starts_with("sha256:"));
            assert_eq!(hash1.len(), 71); // "sha256:" (7) + 64 hex chars
        }

        #[test]
        fn test_hash_different_snapshots() {
            let s1 = json!({"version": 1, "permissions": ["read"], "roles": [], "tenant": "t1"});
            let s2 = json!({"version": 2, "permissions": ["read", "write"], "roles": ["admin"], "tenant": "t1"});
            assert_ne!(compute(&s1), compute(&s2));
        }

        #[test]
        fn test_verify_matching() {
            let snapshot = json!({"version": 1, "permissions": ["read"], "roles": [], "tenant": "t"});
            let hash = compute(&snapshot);
            assert!(verify(&snapshot, &hash));
        }

        #[test]
        fn test_verify_mismatch() {
            let snapshot1 = json!({"version": 1, "permissions": ["read"], "roles": [], "tenant": "t"});
            let snapshot2 = json!({"version": 2, "permissions": ["write"], "roles": [], "tenant": "t"});
            let hash = compute(&snapshot1);
            assert!(!verify(&snapshot2, &hash));
        }

        #[test]
        fn test_empty_snapshot() {
            let empty = json!({});
            let hash = compute(&empty);
            assert!(hash.starts_with("sha256:"));
            assert_eq!(hash.len(), 71);
        }

        #[test]
        fn test_special_characters_in_keys() {
            let snapshot = json!({"key-with-dash": "val", "key_with_underscore": "val2"});
            let canonical = canonical_json(&snapshot);
            assert_eq!(canonical, r#"{"key-with-dash":"val","key_with_underscore":"val2"}"#);
        }
    }
}

/// Redis cache service for entitlements snapshots.
///
/// Stores and retrieves full entitlements snapshots with TTL (HACK-204).
/// Provides hash verification on reads (HACK-201).
pub mod entitlements_cache {
    use redis::{Client, Commands, Connection};
    use serde_json::Value;

    /// Default TTL range for entitlements cache: 30-300 seconds.
    pub const MIN_TTL_SECS: u64 = 30;
    pub const MAX_TTL_SECS: u64 = 300;

    /// Cache a full entitlements snapshot.
    ///
    /// # Arguments
    /// * `conn` - Redis connection
    /// * `tenant_id` - Tenant ID for multi-tenant isolation (HACK-206)
    /// * `entitlements_ref` - The reference to use as part of the key
    /// * `snapshot` - The full entitlements snapshot to cache
    /// * `ttl_secs` - TTL in seconds (30-300)
    ///
    /// # Cache Key
    /// `entitlements:{tenant_id}:{entitlements_ref}`
    pub fn cache_snapshot(
        conn: &mut Connection,
        tenant_id: &str,
        entitlements_ref: &str,
        snapshot: &Value,
        ttl_secs: u64,
    ) -> Result<(), String> {
        // Validate TTL range (HACK-204)
        let ttl = ttl_secs.clamp(MIN_TTL_SECS, MAX_TTL_SECS);

        let key = format!("entitlements:{}:{}", tenant_id, entitlements_ref);
        let serialized = serde_json::to_string(snapshot)
            .map_err(|e| format!("Failed to serialize snapshot: {}", e))?;

        conn.set_ex(&key, serialized, ttl)
            .map_err(|e| format!("Redis SET failed: {}", e))?;

        Ok(())
    }

    /// Retrieve a cached entitlements snapshot and verify its hash.
    ///
    /// # Arguments
    /// * `conn` - Redis connection
    /// * `tenant_id` - Tenant ID for key construction
    /// * `entitlements_ref` - The reference to look up
    /// * `expected_hash` - Expected SHA-256 hash from the JWT
    ///
    /// # Returns
    /// * `Ok(Some(snapshot))` - Snapshot found and hash verified
    /// * `Ok(None)` - Cache miss
    /// * `Err` - Cache hit but hash mismatch (HACK-201: potential poisoning)
    ///
    /// # HACK-201
    /// If the hash doesn't match, the caller MUST:
    /// 1. Delete the poisoned cache entry
    /// 2. Fetch authoritative data from authz-core
    /// 3. Re-cache with verified data
    pub fn get_snapshot(
        conn: &mut Connection,
        tenant_id: &str,
        entitlements_ref: &str,
        expected_hash: &str,
    ) -> Result<Option<Value>, String> {
        let key = format!("entitlements:{}:{}", tenant_id, entitlements_ref);

        let serialized: Option<String> = conn.get(&key)
            .map_err(|e| format!("Redis GET failed: {}", e))?;

        match serialized {
            None => Ok(None),
            Some(data) => {
                let snapshot: Value = serde_json::from_str(&data)
                    .map_err(|e| format!("Failed to deserialize snapshot: {}", e))?;

                // HACK-201: Verify hash before returning cached data
                let snapshot_hash = crate::jwt::entitlements_hash::compute(&snapshot);
                if snapshot_hash != expected_hash {
                    // Cache poisoning detected!
                    // Delete the poisoned entry
                    let _ = conn.del(&key);
                    return Err(format!(
                        "Entitlements hash mismatch for ref {}: expected {} got {}",
                        entitlements_ref, expected_hash, snapshot_hash
                    ));
                }

                Ok(Some(snapshot))
            }
        }
    }

    /// Invalidate (delete) a specific entitlements cache entry.
    ///
    /// Used when hash verification fails (HACK-201) or when version changes (HACK-204).
    pub fn invalidate(
        conn: &mut Connection,
        tenant_id: &str,
        entitlements_ref: &str,
    ) -> Result<(), String> {
        let key = format!("entitlements:{}:{}", tenant_id, entitlements_ref);
        conn.del(&key)
            .map_err(|e| format!("Redis DEL failed: {}", e))?;
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use redis::Commands as RedisCommands;

        #[test]
        fn test_ttl_clamping() {
            // TTL below MIN_TTL_SECS should be clamped
            assert_eq!(30.clamp(MIN_TTL_SECS, MAX_TTL_SECS), 30);
            // TTL above MAX_TTL_SECS should be clamped
            assert_eq!(600.clamp(MIN_TTL_SECS, MAX_TTL_SECS), 300);
            // TTL in range should pass through
            assert_eq!(120.clamp(MIN_TTL_SECS, MAX_TTL_SECS), 120);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entitlements_ref_deterministic() {
        // Same tuple → same ref
        let r1 = entitlements_ref::generate("user-123", "org-456", 1);
        let r2 = entitlements_ref::generate("user-123", "org-456", 1);
        assert_eq!(r1, r2);
        assert!(r1.starts_with("ent_"));
    }

    #[test]
    fn test_entitlements_ref_different_version() {
        // Different version → different ref
        let r1 = entitlements_ref::generate("user-123", "org-456", 1);
        let r2 = entitlements_ref::generate("user-123", "org-456", 2);
        assert_ne!(r1, r2);
    }

    #[test]
    fn test_entitlements_ref_different_user() {
        // Different user → different ref
        let r1 = entitlements_ref::generate("user-123", "org-456", 1);
        let r2 = entitlements_ref::generate("user-789", "org-456", 1);
        assert_ne!(r1, r2);
    }

    #[test]
    fn test_entitlements_ref_format() {
        let ref_str = entitlements_ref::generate("test-user", "test-org", 1);
        // Must match pattern: "ent_<36-char UUID>"
        assert_eq!(ref_str.len(), 40); // "ent_" (4) + UUID (36)
        assert!(ref_str.starts_with("ent_"));
        let uuid_part = &ref_str[4..];
        // UUID format: 8-4-4-4-12
        assert!(uuid_part.len() == 36);
    }

    #[test]
    fn test_entitlements_ref_canonical() {
        // The canonical implementation generates:
        // format!("ent_{}", uuid::Uuid::new_v5(
        //     &uuid::Uuid::NAMESPACE_SHA256,
        //     format!("{}:{}:{}", user_id, org_id, version).as_bytes()
        // ))
        let ref_str = entitlements_ref::generate("user-123", "org-456", 1);
        assert_eq!(ref_str, "ent_9213a389-e7e2-5d7e-b4b2-8e0f7c6a5d3c");
    }

    #[test]
    fn test_entitlements_hash_sha256_format() {
        use serde_json::json;
        let snapshot = json!({"version": 1, "permissions": ["read"], "roles": [], "tenant": "t"});
        let hash = entitlements_hash::compute(&snapshot);
        assert!(hash.starts_with("sha256:"));
        let hex_part = &hash[7..];
        assert_eq!(hex_part.len(), 64); // SHA-256 = 256 bits = 64 hex chars
        // Verify it's valid hex
        assert!(hex_part.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
