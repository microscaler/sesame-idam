//! Redis-backed token version store for atomic INCR/GET operations.
//!
//! Stores and manages per-subject and per-tenant version numbers using Redis
//! keys in the format `authz_ver:{sub}` and `authz_ver:tenant:{tenant_id}`.

use anyhow::{Context, Result};
use redis::Client;
use std::time::{SystemTime, UNIX_EPOCH};

/// Prefix for subject-specific version keys.
pub const SUBJECT_KEY_PREFIX: &str = "authz_ver:";

/// Prefix for tenant-specific version keys.
pub const TENANT_KEY_PREFIX: &str = "authz_ver:tenant:";

/// Generate a subject-specific version key.
#[must_use]
pub fn subject_key(subject: &str) -> String {
    format!("{SUBJECT_KEY_PREFIX}{subject}")
}

/// Generate a tenant-specific version key.
#[must_use]
pub fn tenant_key(tenant_id: &str) -> String {
    format!("{TENANT_KEY_PREFIX}{tenant_id}")
}

/// Configuration for the version store.
#[derive(Debug, Clone)]
pub struct VersionStoreConfig {
    /// Redis connection URL.
    pub redis_url: String,
    /// TTL for subject version keys (seconds).
    pub subject_ttl_secs: u64,
    /// TTL for tenant version keys (seconds).
    pub tenant_ttl_secs: u64,
    /// Minimum allowed TTL (enforced as floor).
    pub min_ttl_secs: u64,
}

/// Redis-backed token version store.
///
/// Provides atomic increment and get operations for subject and tenant
/// version numbers, with TTL management for cache expiration.
#[derive(Clone)]
pub struct VersionStore {
    client: Client,
    subject_ttl: u64,
    tenant_ttl: u64,
}

impl VersionStore {
    /// Create a new version store from a config.
    pub fn new(config: &VersionStoreConfig) -> Result<Self> {
        let client = Client::open(config.redis_url.as_str())
            .context("failed to open Redis client for version store")?;

        let subject_ttl = config.subject_ttl_secs.max(config.min_ttl_secs);
        let tenant_ttl = config.tenant_ttl_secs.max(config.min_ttl_secs);

        Ok(Self {
            client,
            subject_ttl,
            tenant_ttl,
        })
    }

    /// Create a version store with a direct URL string.
    ///
    /// Defaults retain authoritative versions for the full 300-second access-token lifetime plus
    /// the configured 60-second validation leeway. Shorter TTLs can reset a version while an old
    /// token is still accepted.
    pub fn from_url(url: &str) -> Result<Self> {
        Self::new(&VersionStoreConfig {
            redis_url: url.to_string(),
            subject_ttl_secs: 360,
            tenant_ttl_secs: 360,
            min_ttl_secs: 360,
        })
    }

    /// Create a version store from the required `REDIS_URL` environment variable.
    ///
    /// # Errors
    ///
    /// Returns an error when `REDIS_URL` is absent or invalid.
    pub fn from_env() -> Result<Self> {
        let redis_url = std::env::var("REDIS_URL").context("REDIS_URL is required")?;
        Self::from_url(&redis_url)
    }

    /// Get the Redis connection (blocking).
    fn get_conn(&self) -> Result<redis::Connection> {
        self.client
            .get_connection()
            .context("failed to get Redis connection")
    }

    /// Increment the version for a subject and return the new version.
    ///
    /// Uses Redis INCR for atomic increment. Sets the key with a TTL
    /// on the first increment (when it didn't exist).
    pub fn increment_subject(&self, subject: &str) -> Result<u64> {
        let key = subject_key(subject);
        let mut conn = self.get_conn()?;

        use redis::Commands;

        // Use INCRBY 1 for atomic increment (delta 0 would never advance)
        let version: u64 = conn.incr(&key, 1)?;

        // Every bump must remain authoritative until all previously issued tokens have expired.
        let _: () = conn
            .expire(&key, self.subject_ttl as i64)
            .map_err(|e| anyhow::anyhow!("expire failed: {e}"))?;

        Ok(version)
    }

    /// Increment the version for a tenant and return the new version.
    pub fn increment_tenant(&self, tenant_id: &str) -> Result<u64> {
        let key = tenant_key(tenant_id);
        let mut conn = self.get_conn()?;

        use redis::Commands;

        let version: u64 = conn.incr(&key, 1)?;

        let _: () = conn
            .expire(&key, self.tenant_ttl as i64)
            .map_err(|e| anyhow::anyhow!("expire failed: {e}"))?;

        Ok(version)
    }

    /// Get the current version for a subject. Returns 0 if not found.
    pub fn get_subject_version(&self, subject: &str) -> Result<u64> {
        let key = subject_key(subject);
        let mut conn = self.get_conn()?;

        use redis::Commands;
        let version: Option<String> = conn.get(&key).unwrap_or_default();

        Ok(version.and_then(|v| v.parse().ok()).unwrap_or(0))
    }

    /// Get the current version for a tenant. Returns 0 if not found.
    pub fn get_tenant_version(&self, tenant_id: &str) -> Result<u64> {
        let key = tenant_key(tenant_id);
        let mut conn = self.get_conn()?;

        use redis::Commands;
        let version: Option<String> = conn.get(&key).unwrap_or_default();

        Ok(version.and_then(|v| v.parse().ok()).unwrap_or(0))
    }

    /// Read or initialize the version used for a newly issued access token.
    ///
    /// Issuing another token does not itself change authorization state and therefore MUST NOT
    /// invalidate earlier sessions. The key TTL is refreshed so the version remains authoritative
    /// for the complete lifetime of the new token.
    pub fn issue_version(&self, subject: &str) -> Result<(u64, u64)> {
        let key = subject_key(subject);
        let mut conn = self.get_conn()?;
        use redis::Commands;

        let _: bool = conn.set_nx(&key, 1_u64)?;
        let version: u64 = conn.get(&key)?;
        let _: () = conn
            .expire(&key, self.subject_ttl as i64)
            .map_err(|e| anyhow::anyhow!("expire failed: {e}"))?;
        Ok((version, self.subject_ttl))
    }

    /// Delete a key from Redis.
    pub fn delete_key(&self, key: &str) -> Result<()> {
        let mut conn = self.get_conn()?;
        use redis::Commands;
        conn.del::<_, ()>(key)
            .map_err(|e| anyhow::anyhow!("del failed: {e}"))?;
        Ok(())
    }

    /// Check if a key exists in Redis.
    pub fn key_exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.get_conn()?;
        use redis::Commands;
        conn.exists(key)
            .map_err(|e| anyhow::anyhow!("redis exists failed: {e}"))
    }

    /// Get the remaining TTL for a key in seconds.
    /// Returns 0 if the key doesn't exist.
    pub fn get_ttl(&self, key: &str) -> Result<u64> {
        let mut conn = self.get_conn()?;
        use redis::Commands;
        let ttl: i64 = conn.ttl(key)?;
        Ok(if ttl > 0 { ttl as u64 } else { 0 })
    }

    /// Delete specific keys for testing/cleanup.
    ///
    /// For production use, use dedicated test namespaces or key prefixes
    /// rather than deleting arbitrary keys.
    pub fn flush_keys(&self, keys: &[&str]) -> Result<()> {
        let mut conn = self.get_conn()?;
        use redis::Commands;
        for key in keys {
            conn.del::<_, ()>(key)
                .map_err(|e| anyhow::anyhow!("del failed: {e}"))?;
        }
        Ok(())
    }

    /// Get the subject TTL (in seconds).
    #[must_use]
    pub fn subject_ttl(&self) -> u64 {
        self.subject_ttl
    }

    /// Get the tenant TTL (in seconds).
    #[must_use]
    pub fn tenant_ttl(&self) -> u64 {
        self.tenant_ttl
    }

    /// Wrapper around the free function `subject_key` for use in tests.
    #[doc(hidden)]
    #[must_use]
    pub fn subject_key(&self, subject: &str) -> String {
        subject_key(subject)
    }

    /// Wrapper around the free function `tenant_key` for use in tests.
    #[doc(hidden)]
    #[must_use]
    pub fn tenant_key(&self, tenant_id: &str) -> String {
        tenant_key(tenant_id)
    }

    /// Get the current Unix timestamp in seconds.
    #[must_use]
    pub fn current_unix_seconds() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;

    fn test_redis_url() -> String {
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into())
    }

    fn redis_available() -> bool {
        let url = test_redis_url();
        let client = Client::open(url.as_str());
        match client {
            Ok(c) => c.get_connection().is_ok(),
            Err(_) => false,
        }
    }

    fn unique_key(prefix: &str) -> String {
        format!(
            "{}:{}{}",
            prefix,
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            std::process::id()
        )
    }

    #[test]
    fn test_increment_subject_returns_sequential() {
        if !redis_available() {
            println!("SKIP: Redis not available");
            return;
        }

        let user = unique_key("test_seq");
        let store = VersionStore::from_url(&test_redis_url()).unwrap();
        let key = store.subject_key(&user);

        // Clean up before test
        store.delete_key(&key);

        let v1 = store.increment_subject(&user).unwrap();
        assert_eq!(v1, 1);

        let v2 = store.increment_subject(&user).unwrap();
        assert_eq!(v2, 2);

        let v3 = store.increment_subject(&user).unwrap();
        assert_eq!(v3, 3);

        assert!(v1 < v2 && v2 < v3);

        // Clean up
        store.delete_key(&key);
    }

    #[test]
    fn test_get_subject_version_returns_zero_for_new_user() {
        if !redis_available() {
            println!("SKIP: Redis not available");
            return;
        }

        let store = VersionStore::from_url(&test_redis_url()).unwrap();
        let user = format!(
            "test_new_{}{}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            std::process::id()
        );
        let key = store.subject_key(&user);

        let ver = store.get_subject_version(&user).unwrap();
        assert_eq!(ver, 0);

        // Clean up
        store.delete_key(&key);
    }

    #[test]
    fn test_get_subject_version_after_increment() {
        if !redis_available() {
            println!("SKIP: Redis not available");
            return;
        }

        let store = VersionStore::from_url(&test_redis_url()).unwrap();
        let user = format!(
            "test_get_{}{}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            std::process::id()
        );
        let key = store.subject_key(&user);

        store.increment_subject(&user).unwrap();

        let ver = store.get_subject_version(&user).unwrap();
        assert_eq!(ver, 1);

        store.delete_key(&key);
    }

    #[test]
    fn test_increment_tenant_returns_sequential() {
        if !redis_available() {
            println!("SKIP: Redis not available");
            return;
        }

        let store = VersionStore::from_url(&test_redis_url()).unwrap();
        let tenant = format!(
            "test_tenant_{}{}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            std::process::id()
        );
        let key = store.tenant_key(&tenant);

        store.delete_key(&key);

        let v1 = store.increment_tenant(&tenant).unwrap();
        assert_eq!(v1, 1);

        let v2 = store.increment_tenant(&tenant).unwrap();
        assert_eq!(v2, 2);

        store.delete_key(&key);
    }

    #[test]
    fn test_get_tenant_version_returns_zero_for_new_tenant() {
        if !redis_available() {
            println!("SKIP: Redis not available");
            return;
        }

        let store = VersionStore::from_url(&test_redis_url()).unwrap();
        let tenant = format!(
            "test_newtenant_{}{}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            std::process::id()
        );

        let ver = store.get_tenant_version(&tenant).unwrap();
        assert_eq!(ver, 0);

        store.delete_key(&store.tenant_key(&tenant));
    }

    #[test]
    fn test_independent_subject_and_tenant_versions() {
        if !redis_available() {
            println!("SKIP: Redis not available");
            return;
        }

        let store = VersionStore::from_url(&test_redis_url()).unwrap();
        let user = unique_key("test_indep_user");
        let tenant = unique_key("test_indep_tenant");

        // Increment user version
        store.increment_subject(&user).unwrap();
        // Increment tenant version independently
        store.increment_tenant(&tenant).unwrap();

        // User should be at version 1
        let user_ver = store.get_subject_version(&user).unwrap();
        assert_eq!(user_ver, 1);

        // Tenant should be at version 1
        let tenant_ver = store.get_tenant_version(&tenant).unwrap();
        assert_eq!(tenant_ver, 1);

        // Increment tenant — user version should NOT change
        store.increment_tenant(&tenant).unwrap();
        let tenant_ver_after = store.get_tenant_version(&tenant).unwrap();
        assert_eq!(tenant_ver_after, 2);

        let user_ver_after = store.get_subject_version(&user).unwrap();
        assert_eq!(user_ver_after, 1); // unchanged

        store.delete_key(&store.subject_key(&user));
        store.delete_key(&store.tenant_key(&tenant));
    }

    #[test]
    fn test_ttl_is_set_on_increment() {
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
        let user = unique_key("test_ttl");

        store.increment_subject(&user).unwrap();

        let ttl = store.get_ttl(&store.subject_key(&user)).unwrap();
        // TTL should be close to 30 (within 5 seconds due to timing)
        assert!(ttl > 20 && ttl <= 30, "TTL was {ttl}s, expected ~30s");

        store.delete_key(&store.subject_key(&user));
    }

    #[test]
    fn test_issue_version_returns_tuple() {
        if !redis_available() {
            println!("SKIP: Redis not available");
            return;
        }

        let store = VersionStore::from_url(&test_redis_url()).unwrap();
        let user = format!(
            "test_issue_{}{}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            std::process::id()
        );
        let key = store.subject_key(&user);

        store.delete_key(&key);

        let (ver, ttl) = store.issue_version(&user).unwrap();
        assert_eq!(ver, 1);
        assert_eq!(ttl, 360); // access TTL + validation leeway

        let (second_ver, _) = store.issue_version(&user).unwrap();
        assert_eq!(second_ver, 1, "issuing a token must not bump authz state");

        store.delete_key(&key);
    }

    #[test]
    fn test_key_exists_true_after_increment() {
        if !redis_available() {
            println!("SKIP: Redis not available");
            return;
        }

        let store = VersionStore::from_url(&test_redis_url()).unwrap();
        let user = format!(
            "test_exists_{}{}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            std::process::id()
        );
        let key = store.subject_key(&user);

        store.delete_key(&key);

        assert!(!store.key_exists(&key).unwrap());

        store.increment_subject(&user).unwrap();
        assert!(store.key_exists(&key).unwrap());

        store.delete_key(&key);
    }

    #[test]
    fn test_concurrent_increments_no_duplicates() {
        if !redis_available() {
            println!("SKIP: Redis not available");
            return;
        }

        let store = VersionStore::from_url(&test_redis_url()).unwrap();
        let user = unique_key("test_concurrent");
        let key = store.subject_key(&user);

        store.delete_key(&key);

        // Run 10 concurrent increments using std::thread
        let mut handles = vec![];
        for _ in 0..10 {
            let store_clone = store.clone();
            let user_clone = user.clone();
            handles.push(std::thread::spawn(move || {
                store_clone.increment_subject(&user_clone).unwrap()
            }));
        }

        let versions: Vec<u64> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        let mut sorted_versions = versions.clone();
        sorted_versions.sort_unstable();

        // Should be 1 through 10, no duplicates
        assert_eq!(sorted_versions.len(), 10);
        for (i, v) in sorted_versions.iter().enumerate() {
            assert_eq!(*v, (i + 1) as u64, "Expected {} but got {}", i + 1, v);
        }

        store.delete_key(&key);
    }

    #[test]
    fn test_flush_keys_cleans_targeted_keys() {
        if !redis_available() {
            println!("SKIP: Redis not available");
            return;
        }

        let store = VersionStore::from_url(&test_redis_url()).unwrap();
        let user = unique_key("test_flush_user");
        let tenant = unique_key("test_flush_tenant");
        let user_key = store.subject_key(&user);
        let tenant_key = store.tenant_key(&tenant);

        // Create some keys
        store.increment_subject(&user).unwrap();
        store.increment_tenant(&tenant).unwrap();

        assert!(store.key_exists(&user_key).unwrap());
        assert!(store.key_exists(&tenant_key).unwrap());

        // Delete specific keys (not FLUSHDB, which would wipe other tests' data)
        store.flush_keys(&[&user_key, &tenant_key]).unwrap();

        // After deleting, versions should be 0
        let user_ver = store.get_subject_version(&user).unwrap();
        let tenant_ver = store.get_tenant_version(&tenant).unwrap();
        assert_eq!(user_ver, 0);
        assert_eq!(tenant_ver, 0);
    }

    #[test]
    fn test_monotonically_increasing_across_calls() {
        if !redis_available() {
            println!("SKIP: Redis not available");
            return;
        }

        let store = VersionStore::from_url(&test_redis_url()).unwrap();
        let user = unique_key("test_mono");
        let key = store.subject_key(&user);

        store.delete_key(&key);

        let mut last_ver = 0u64;
        for i in 1..=20 {
            let ver = store.increment_subject(&user).unwrap();
            assert!(
                ver > last_ver,
                "Version {ver} was not greater than {last_ver}"
            );
            assert_eq!(ver, i, "Expected {i} but got {ver}");
            last_ver = ver;
        }

        store.delete_key(&key);
    }
}
