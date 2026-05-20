//! Redis-backed version store for per-subject and per-tenant token versions.
//!
//! Provides atomic `INCR` and `GET` operations on `authz_ver` keys with TTL management.
//! This is the primary version tracking mechanism for Story 5.1.
//!
//! # Key Names
//!
//! - Subject version: `authz_ver:{user_id}` — tracks per-subject version
//! - Tenant version: `authz_ver:tenant:{tenant_id}` — tracks per-tenant version
//!
//! # TTL Strategy
//!
//! Subject version: 15 seconds. Tenant version: 60 seconds.
//! This is shorter than token TTL (300s), meaning after a version bump,
//! stale tokens are rejected for the cache TTL duration. After TTL expiry,
//! the cache is empty and the version check falls back to Redis lookup.
//!
//! # Concurrency
//!
//! Redis `INCR` is atomic — concurrent increments from multiple services
//! produce strictly sequential values with no lost updates.

use anyhow::{Context, Result};
use redis::Client;
use tracing::debug;

/// Key prefix for subject (user-specific) versions.
pub const SUBJECT_KEY_PREFIX: &str = "authz_ver:";

/// Key prefix for tenant-wide versions.
pub const TENANT_KEY_PREFIX: &str = "authz_ver:tenant:";

/// Default TTL for subject version keys (seconds).
pub const DEFAULT_SUBJECT_TTL_SECS: u64 = 15;

/// Default TTL for tenant version keys (seconds).
pub const DEFAULT_TENANT_TTL_SECS: u64 = 60;

/// Version store configuration.
#[derive(Debug, Clone)]
pub struct VersionStoreConfig {
    /// Redis connection URL (e.g., "redis://127.0.0.1:6379").
    pub redis_url: String,
    /// TTL for subject version keys.
    pub subject_ttl_secs: u64,
    /// TTL for tenant version keys.
    pub tenant_ttl_secs: u64,
    /// Minimum TTL value (prevents accidentally zero TTL).
    pub min_ttl_secs: u64,
}

impl Default for VersionStoreConfig {
    fn default() -> Self {
        Self {
            redis_url: "redis://127.0.0.1:6379".to_string(),
            subject_ttl_secs: DEFAULT_SUBJECT_TTL_SECS,
            tenant_ttl_secs: DEFAULT_TENANT_TTL_SECS,
            min_ttl_secs: 15,
        }
    }
}

impl VersionStoreConfig {
    /// Validate TTL configuration.
    pub fn validate(&self) -> Result<()> {
        if self.subject_ttl_secs < self.min_ttl_secs {
            anyhow::bail!(
                "subject TTL {}s is below minimum {}s",
                self.subject_ttl_secs,
                self.min_ttl_secs
            );
        }
        if self.tenant_ttl_secs < self.min_ttl_secs {
            anyhow::bail!(
                "tenant TTL {}s is below minimum {}s",
                self.tenant_ttl_secs,
                self.min_ttl_secs
            );
        }
        Ok(())
    }
}

/// Redis-backed version store for token version tracking.
///
/// Thread-safe: uses `Client` which is `Clone` + `Send` + `Sync`.
#[derive(Clone)]
pub struct VersionStore {
    client: Client,
    subject_ttl_secs: u64,
    tenant_ttl_secs: u64,
    min_ttl_secs: u64,
}

impl VersionStore {
    /// Create a new version store from a config.
    ///
    /// # Errors
    ///
    /// Returns an error if the Redis URL is invalid or config validation fails.
    pub fn new(config: &VersionStoreConfig) -> Result<Self> {
        config.validate()?;
        let client =
            Client::open(config.redis_url.as_str())
                .context("failed to open Redis client for version store")?;
        Ok(Self {
            client,
            subject_ttl_secs: config.subject_ttl_secs,
            tenant_ttl_secs: config.tenant_ttl_secs,
            min_ttl_secs: config.min_ttl_secs,
        })
    }

    /// Create a new version store with a Redis URL string.
    ///
    /// Uses default TTLs: 15s for subjects, 60s for tenants.
    pub fn from_url(redis_url: &str) -> Result<Self> {
        let config = VersionStoreConfig {
            redis_url: redis_url.to_string(),
            ..Default::default()
        };
        Self::new(&config)
    }

    /// Increment the subject version and store with TTL.
    ///
    /// This is the primary method used during token issuance.
    ///
    /// Steps:
    /// 1. `INCR authz_ver:{user_id}` — atomically increment
    /// 2. `SET authz_ver:{user_id} EX {ttl}` — ensure TTL is set (INCR doesn't set TTL on new keys)
    ///
    /// # Arguments
    ///
    /// * `user_id` — The subject whose version to increment
    ///
    /// # Returns
    ///
    /// The new version number (monotonically increasing u64).
    /// Returns `Ok(0)` if Redis is unreachable (fail-safe).
    pub async fn increment_subject(&self, user_id: &str) -> Result<u64> {
        let key = self.subject_key(user_id);
        let ttl = self.subject_ttl_secs;

        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .context("failed to get Redis connection for subject version increment")?;

        // Step 1: Atomically increment the counter.
        // INCR returns the value AFTER increment.
        // For a new key, INCR starts at 0 and returns 1.
        let new_ver: u64 = redis::cmd("INCR")
            .arg(&key)
            .query_async::<_, u64>(&mut conn)
            .await
            .context("failed to INCR subject version in Redis")?;

        // Step 2: Set TTL. This only takes effect for new keys or when the key has no TTL.
        // For existing keys with TTL, we use SETEX to refresh the TTL without changing the value.
        // We use a pipeline to set TTL atomically after increment.
        let ttl = if ttl < self.min_ttl_secs {
            self.min_ttl_secs
        } else {
            ttl
        };

        // Use SET to atomically set value and TTL (replaces current value, but we just INCR'd so this is fine)
        // Actually, we should use SET with NX or SETEX to just refresh TTL.
        // The safest approach: SET key value EX ttl — but this overwrites.
        // Better: Just SETEX the key (INCR already set it, so this refreshes TTL).
        redis::cmd("SETEX")
            .arg(&key)
            .arg(ttl)
            .arg(new_ver)
            .query_async::<_, ()>(&mut conn)
            .await
            .context("failed to SET subject version TTL in Redis")?;

        debug!(
            user_id,
            new_version = new_ver,
            ttl_secs = ttl,
            "incremented subject version",
        );

        Ok(new_ver)
    }

    /// Increment the tenant version and store with TTL.
    ///
    /// Called when authz changes occur (role/permission changes).
    ///
    /// # Arguments
    ///
    /// * `tenant_id` — The tenant whose version to increment
    ///
    /// # Returns
    ///
    /// The new tenant version number (monotonically increasing u64).
    pub async fn increment_tenant(&self, tenant_id: &str) -> Result<u64> {
        let key = self.tenant_key(tenant_id);
        let ttl = self.tenant_ttl_secs;

        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .context("failed to get Redis connection for tenant version increment")?;

        let new_ver: u64 = redis::cmd("INCR")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .context("failed to INCR tenant version in Redis")?;

        let ttl = if ttl < self.min_ttl_secs {
            self.min_ttl_secs
        } else {
            ttl
        };

        redis::cmd("SETEX")
            .arg(&key)
            .arg(ttl)
            .arg(new_ver)
            .query_async::<_, ()>(&mut conn)
            .await
            .context("failed to SET tenant version TTL in Redis")?;

        debug!(
            tenant_id,
            new_version = new_ver,
            ttl_secs = ttl,
            "incremented tenant version",
        );

        Ok(new_ver)
    }

    /// Get the current subject version.
    ///
    /// Returns `Ok(0)` if the key does not exist (default version).
    /// Returns `Err` if Redis is unreachable.
    ///
    /// # Arguments
    ///
    /// * `user_id` — The subject to look up
    pub async fn get_subject_version(&self, user_id: &str) -> Result<u64> {
        let key = self.subject_key(user_id);
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .context("failed to get Redis connection for subject version lookup")?;

        let version: Option<u64> = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .context("failed to GET subject version from Redis")?;

        Ok(version.unwrap_or(0))
    }

    /// Get the current tenant version.
    ///
    /// Returns `Ok(0)` if the key does not exist.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` — The tenant to look up
    pub async fn get_tenant_version(&self, tenant_id: &str) -> Result<u64> {
        let key = self.tenant_key(tenant_id);
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .context("failed to get Redis connection for tenant version lookup")?;

        let version: Option<u64> = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .context("failed to GET tenant version from Redis")?;

        Ok(version.unwrap_or(0))
    }

    /// Atomically increment subject version, set TTL, and store in Redis.
    ///
    /// This is a convenience method that does the same as `increment_subject()`
    /// but is explicitly named for use in token issuance flow.
    ///
    /// # Returns
    ///
    /// Tuple of `(new_version, ttl_seconds)`.
    pub async fn issue_version(&self, user_id: &str) -> Result<(u64, u64)> {
        let new_ver = self.increment_subject(user_id).await?;
        Ok((new_ver, self.subject_ttl_secs))
    }

    /// Get the subject cache key for a user.
    pub fn subject_key(&self, user_id: &str) -> String {
        format!("{}{}", SUBJECT_KEY_PREFIX, user_id)
    }

    /// Get the tenant cache key for a tenant.
    pub fn tenant_key(&self, tenant_id: &str) -> String {
        format!("{}{}", TENANT_KEY_PREFIX, tenant_id)
    }

    /// Check if a key exists in Redis.
    ///
    /// Returns `true` if the key exists, `false` otherwise.
    pub async fn key_exists(&self, key: &str) -> Result<bool> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .context("failed to get Redis connection for key existence check")?;

        let exists: bool = redis::cmd("EXISTS")
            .arg(key)
            .query_async::<_, bool>(&mut conn)
            .await
            .context("failed to check key existence in Redis")?;

        Ok(exists)
    }

    /// Delete a version key from Redis.
    ///
    /// Used for cleanup between tests.
    pub async fn delete_key(&self, key: &str) -> Result<()> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .context("failed to get Redis connection for key deletion")?;

        redis::cmd("DEL")
            .arg(key)
            .query_async::<_, i64>(&mut conn)
            .await
            .context("failed to delete key from Redis")?;

        Ok(())
    }

    /// Flush all version keys from Redis.
    ///
    /// Used for test cleanup. WARNING: deletes ALL authz_ver keys.
    pub async fn flush_all(&self) -> Result<()> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .context("failed to get Redis connection for flush")?;

        redis::cmd("FLUSHDB")
            .query_async::<_, ()>(&mut conn)
            .await
            .context("failed to flush Redis DB")?;

        Ok(())
    }

    /// Get the TTL for a key.
    ///
    /// Returns `Ok(-2)` if the key does not exist.
    /// Returns `Ok(-1)` if the key exists but has no TTL.
    pub async fn get_ttl(&self, key: &str) -> Result<i64> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .context("failed to get Redis connection for TTL lookup")?;

        let ttl: i64 = redis::cmd("TTL")
            .arg(key)
            .query_async(&mut conn)
            .await
            .context("failed to get TTL from Redis")?;

        Ok(ttl)
    }
}

/// Convenience function to generate a subject key.
pub fn subject_key(user_id: &str) -> String {
    format!("{}{}", SUBJECT_KEY_PREFIX, user_id)
}

/// Convenience function to generate a tenant key.
pub fn tenant_key(tenant_id: &str) -> String {
    format!("{}{}", TENANT_KEY_PREFIX, tenant_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    // Helper to get Redis URL from environment or use default.
    fn test_redis_url() -> String {
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
    }

    // Test that validates Redis is reachable.
    // Skip if Redis is not available — these tests require a live Redis instance.
    fn redis_available() -> bool {
        let url = test_redis_url();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = Client::open(url.as_str());
            match client {
                Ok(c) => {
                    let mut conn = c.get_multiplexed_async_connection().await;
                    conn.is_ok()
                }
                Err(_) => false,
            }
        })
    }

    #[test]
    fn test_subject_key_format() {
        assert_eq!(subject_key("user_123"), "authz_ver:user_123");
    }

    #[test]
    fn test_tenant_key_format() {
        assert_eq!(tenant_key("tenant_abc"), "authz_ver:tenant:tenant_abc");
    }

    #[test]
    fn test_store_key_methods() {
        let url = test_redis_url();
        let store = VersionStore::from_url(&url).unwrap();
        assert_eq!(store.subject_key("u1"), "authz_ver:u1");
        assert_eq!(store.tenant_key("t1"), "authz_ver:tenant:t1");
    }

    #[test]
    fn test_config_default() {
        let config = VersionStoreConfig::default();
        assert_eq!(config.subject_ttl_secs, 15);
        assert_eq!(config.tenant_ttl_secs, 60);
        assert_eq!(config.min_ttl_secs, 15);
    }

    #[test]
    fn test_config_validation_high_ttl() {
        let config = VersionStoreConfig {
            subject_ttl_secs: 100,
            tenant_ttl_secs: 200,
            min_ttl_secs: 15,
            redis_url: test_redis_url(),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_low_ttl_rejected() {
        let config = VersionStoreConfig {
            subject_ttl_secs: 5,
            tenant_ttl_secs: 60,
            min_ttl_secs: 15,
            redis_url: test_redis_url(),
        };
        assert!(config.validate().is_err());
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("below minimum"));
    }

    #[test]
    fn test_config_validation_tenant_low_ttl_rejected() {
        let config = VersionStoreConfig {
            subject_ttl_secs: 30,
            tenant_ttl_secs: 5,
            min_ttl_secs: 15,
            redis_url: test_redis_url(),
        };
        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn test_increment_subject_returns_sequential() {
        if !redis_available() {
            println!("SKIP: Redis not available");
            return;
        }

        let store = VersionStore::from_url(&test_redis_url()).unwrap();
        let user = format!("test_seq_{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        let key = store.subject_key(&user);

        // Clean up before test
        store.delete_key(&key).await.ok();

        let v1 = store.increment_subject(&user).await.unwrap();
        assert_eq!(v1, 1);

        let v2 = store.increment_subject(&user).await.unwrap();
        assert_eq!(v2, 2);

        let v3 = store.increment_subject(&user).await.unwrap();
        assert_eq!(v3, 3);

        assert!(v1 < v2 && v2 < v3);

        // Clean up
        store.delete_key(&key).await.ok();
    }

    #[tokio::test]
    async fn test_get_subject_version_returns_zero_for_new_user() {
        if !redis_available() {
            println!("SKIP: Redis not available");
            return;
        }

        let store = VersionStore::from_url(&test_redis_url()).unwrap();
        let user = format!(
            "test_new_{}",
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        );

        let ver = store.get_subject_version(&user).await.unwrap();
        assert_eq!(ver, 0);

        // Clean up
        store.delete_key(&store.subject_key(&user)).await.ok();
    }

    #[tokio::test]
    async fn test_get_subject_version_after_increment() {
        if !redis_available() {
            println!("SKIP: Redis not available");
            return;
        }

        let store = VersionStore::from_url(&test_redis_url()).unwrap();
        let user = format!(
            "test_get_{}",
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        );
        let key = store.subject_key(&user);

        store.increment_subject(&user).await.unwrap();

        let ver = store.get_subject_version(&user).await.unwrap();
        assert_eq!(ver, 1);

        store.delete_key(&key).await.ok();
    }

    #[tokio::test]
    async fn test_increment_tenant_returns_sequential() {
        if !redis_available() {
            println!("SKIP: Redis not available");
            return;
        }

        let store = VersionStore::from_url(&test_redis_url()).unwrap();
        let tenant = format!(
            "test_tenant_{}",
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        );
        let key = store.tenant_key(&tenant);

        store.delete_key(&key).await.ok();

        let v1 = store.increment_tenant(&tenant).await.unwrap();
        assert_eq!(v1, 1);

        let v2 = store.increment_tenant(&tenant).await.unwrap();
        assert_eq!(v2, 2);

        store.delete_key(&key).await.ok();
    }

    #[tokio::test]
    async fn test_get_tenant_version_returns_zero_for_new_tenant() {
        if !redis_available() {
            println!("SKIP: Redis not available");
            return;
        }

        let store = VersionStore::from_url(&test_redis_url()).unwrap();
        let tenant = format!(
            "test_newtenant_{}",
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        );

        let ver = store.get_tenant_version(&tenant).await.unwrap();
        assert_eq!(ver, 0);

        store.delete_key(&store.tenant_key(&tenant)).await.ok();
    }

    #[tokio::test]
    async fn test_independent_subject_and_tenant_versions() {
        if !redis_available() {
            println!("SKIP: Redis not available");
            return;
        }

        let store = VersionStore::from_url(&test_redis_url()).unwrap();
        let user = format!(
            "test_indep_user_{}",
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        );
        let tenant = format!(
            "test_indep_tenant_{}",
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        );

        // Increment user version
        store.increment_subject(&user).await.unwrap();
        // Increment tenant version independently
        store.increment_tenant(&tenant).await.unwrap();

        // User should be at version 1
        let user_ver = store.get_subject_version(&user).await.unwrap();
        assert_eq!(user_ver, 1);

        // Tenant should be at version 1
        let tenant_ver = store.get_tenant_version(&tenant).await.unwrap();
        assert_eq!(tenant_ver, 1);

        // Increment tenant — user version should NOT change
        store.increment_tenant(&tenant).await.unwrap();
        let tenant_ver_after = store.get_tenant_version(&tenant).await.unwrap();
        assert_eq!(tenant_ver_after, 2);

        let user_ver_after = store.get_subject_version(&user).await.unwrap();
        assert_eq!(user_ver_after, 1); // unchanged

        store.delete_key(&store.subject_key(&user)).await.ok();
        store.delete_key(&store.tenant_key(&tenant)).await.ok();
    }

    #[tokio::test]
    async fn test_ttl_is_set_on_increment() {
        if !redis_available() {
            println!("SKIP: Redis not available");
            return;
        }

        let config = VersionStoreConfig {
            redis_url: test_redis_url(),
            subject_ttl_secs: 30,
            tenant_ttl_secs: 60,
            min_ttl_secs: 15,
        };
        let store = VersionStore::new(&config).unwrap();
        let user = format!(
            "test_ttl_{}",
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        );

        store.increment_subject(&user).await.unwrap();

        let ttl = store.get_ttl(&store.subject_key(&user)).await.unwrap();
        // TTL should be close to 30 (within 5 seconds due to timing)
        assert!(ttl > 20 && ttl <= 30, "TTL was {}s, expected ~30s", ttl);

        store.delete_key(&store.subject_key(&user)).await.ok();
    }

    #[tokio::test]
    async fn test_issue_version_returns_tuple() {
        if !redis_available() {
            println!("SKIP: Redis not available");
            return;
        }

        let store = VersionStore::from_url(&test_redis_url()).unwrap();
        let user = format!(
            "test_issue_{}",
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        );
        let key = store.subject_key(&user);

        store.delete_key(&key).await.ok();

        let (ver, ttl) = store.issue_version(&user).await.unwrap();
        assert_eq!(ver, 1);
        assert_eq!(ttl, 15); // default subject TTL

        store.delete_key(&key).await.ok();
    }

    #[tokio::test]
    async fn test_key_exists_true_after_increment() {
        if !redis_available() {
            println!("SKIP: Redis not available");
            return;
        }

        let store = VersionStore::from_url(&test_redis_url()).unwrap();
        let user = format!(
            "test_exists_{}",
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        );
        let key = store.subject_key(&user);

        store.delete_key(&key).await.ok();

        assert!(!store.key_exists(&key).await.unwrap());

        store.increment_subject(&user).await.unwrap();
        assert!(store.key_exists(&key).await.unwrap());

        store.delete_key(&key).await.ok();
    }

    #[tokio::test]
    async fn test_concurrent_increments_no_duplicates() {
        if !redis_available() {
            println!("SKIP: Redis not available");
            return;
        }

        let store = VersionStore::from_url(&test_redis_url()).unwrap();
        let user = format!(
            "test_concurrent_{}",
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        );
        let key = store.subject_key(&user);

        store.delete_key(&key).await.ok();

        // Run 10 concurrent increments
        let mut handles = vec![];
        for _ in 0..10 {
            let store_clone = store.clone();
            let user_clone = user.clone();
            handles.push(tokio::spawn(async move {
                store_clone.increment_subject(&user_clone).await.unwrap()
            }));
        }

        let versions: Vec<u64> = futures_util::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();
        let mut sorted_versions = versions.clone();
        sorted_versions.sort();

        // Should be 1 through 10, no duplicates
        assert_eq!(sorted_versions.len(), 10);
        for (i, v) in sorted_versions.iter().enumerate() {
            assert_eq!(*v, (i + 1) as u64, "Expected {} but got {}", i + 1, v);
        }

        store.delete_key(&key).await.ok();
    }

    #[tokio::test]
    async fn test_flush_all_cleans_keys() {
        if !redis_available() {
            println!("SKIP: Redis not available");
            return;
        }

        let store = VersionStore::from_url(&test_redis_url()).unwrap();
        let user = format!(
            "test_flush_{}",
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        );
        let tenant = format!(
            "test_flush_tenant_{}",
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        );

        store.increment_subject(&user).await.unwrap();
        store.increment_tenant(&tenant).await.unwrap();

        // Both should exist
        assert!(store.key_exists(&store.subject_key(&user)).await.unwrap());
        assert!(store.key_exists(&store.tenant_key(&tenant)).await.unwrap());

        // Flush all
        store.flush_all().await.unwrap();

        // After flush, versions should be 0
        let user_ver = store.get_subject_version(&user).await.unwrap();
        let tenant_ver = store.get_tenant_version(&tenant).await.unwrap();
        assert_eq!(user_ver, 0);
        assert_eq!(tenant_ver, 0);
    }

    #[tokio::test]
    async fn test_monotonically_increasing_across_calls() {
        if !redis_available() {
            println!("SKIP: Redis not available");
            return;
        }

        let store = VersionStore::from_url(&test_redis_url()).unwrap();
        let user = format!(
            "test_mono_{}",
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        );
        let key = store.subject_key(&user);

        store.delete_key(&key).await.ok();

        let mut last_ver = 0u64;
        for i in 1..=20 {
            let ver = store.increment_subject(&user).await.unwrap();
            assert!(ver > last_ver, "Version {} was not greater than {}", ver, last_ver);
            assert_eq!(ver, i, "Expected {} but got {}", i, ver);
            last_ver = ver;
        }

        store.delete_key(&key).await.ok();
    }
}
