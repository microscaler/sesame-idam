//! Configuration for the denylist cache.
//!
//! Controls TTL behavior, size limits, and Redis connection.

use std::time::Duration;

/// Configuration for the denylist cache.
///
/// # Default values
///
/// - `max_entries`: 10,000 (per service instance)
/// - `max_ttl`: 5 minutes (hard cap on cache entry lifetime)
/// - `default_ttl_secs`: 300 seconds (when token exp is unavailable)
/// - `jitter_factor`: 0.2 (20% random jitter on TTL)
/// - `redis_key_prefix`: "denylist" (Redis key prefix)
/// - `redis_url`: "redis://127.0.0.1:6379"
#[derive(Debug, Clone)]
pub struct DenylistConfig {
    /// Maximum number of entries in the local cache per service instance.
    /// When exceeded, the oldest entries are evicted.
    pub max_entries: usize,
    /// Hard cap on cache entry lifetime in seconds.
    /// Even if a token's `exp` is further in the future, the cache
    /// entry expires after this duration.
    pub max_ttl_secs: u64,
    /// Default TTL in seconds when token expiry is unavailable.
    pub default_ttl_secs: u64,
    /// Jitter factor for TTL randomization (0.0 to 1.0).
    /// Actual TTL = calculated_ttl * (1.0 - jitter + 2.0 * jitter * random).
    /// A factor of 0.2 means TTL varies between 60% and 140% of calculated.
    pub jitter_factor: f64,
    /// Redis key prefix for denylist entries.
    pub redis_key_prefix: String,
    /// Redis connection URL.
    pub redis_url: String,
}

impl Default for DenylistConfig {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            max_ttl_secs: 300, // 5 minutes
            default_ttl_secs: 300,
            jitter_factor: 0.2,
            redis_key_prefix: "denylist".to_string(),
            redis_url: "redis://127.0.0.1:6379".to_string(),
        }
    }
}

impl DenylistConfig {
    /// Create a new configuration with custom values.
    pub fn new(
        max_entries: usize,
        max_ttl_secs: u64,
        default_ttl_secs: u64,
        jitter_factor: f64,
    ) -> Self {
        Self {
            max_entries,
            max_ttl_secs,
            default_ttl_secs,
            jitter_factor,
            redis_key_prefix: "denylist".to_string(),
            redis_url: "redis://127.0.0.1:6379".to_string(),
        }
    }

    /// Set the maximum number of entries.
    pub fn with_max_entries(mut self, max_entries: usize) -> Self {
        self.max_entries = max_entries;
        self
    }

    /// Set the maximum TTL in seconds.
    pub fn with_max_ttl_secs(mut self, max_ttl_secs: u64) -> Self {
        self.max_ttl_secs = max_ttl_secs;
        self
    }

    /// Set the default TTL when token exp is unavailable.
    pub fn with_default_ttl_secs(mut self, default_ttl_secs: u64) -> Self {
        self.default_ttl_secs = default_ttl_secs;
        self
    }

    /// Set the jitter factor (0.0 to 1.0).
    pub fn with_jitter_factor(mut self, jitter_factor: f64) -> Self {
        self.jitter_factor = jitter_factor;
        self
    }

    /// Set the Redis key prefix.
    pub fn with_redis_key_prefix(mut self, prefix: &str) -> Self {
        self.redis_key_prefix = prefix.to_string();
        self
    }

    /// Set the Redis connection URL.
    pub fn with_redis_url(mut self, url: &str) -> Self {
        self.redis_url = url.to_string();
        self
    }

    /// Apply configuration from environment variables.
    ///
    /// Environment variables:
    /// - `DENYLIST_MAX_ENTRIES`: maximum cache entries (default: 10000)
    /// - `DENYLIST_MAX_TTL_SECS`: hard cap TTL in seconds (default: 300)
    /// - `DENYLIST_DEFAULT_TTL_SECS`: default TTL when exp unavailable (default: 300)
    /// - `DENYLIST_JITTER_FACTOR`: jitter factor 0.0-1.0 (default: 0.2)
    /// - `DENYLIST_REDIS_URL`: Redis connection URL (default: redis://127.0.0.1:6379)
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(max_entries) = std::env::var("DENYLIST_MAX_ENTRIES") {
            if let Ok(val) = max_entries.parse::<usize>() {
                config.max_entries = val;
            }
        }

        if let Ok(max_ttl) = std::env::var("DENYLIST_MAX_TTL_SECS") {
            if let Ok(val) = max_ttl.parse::<u64>() {
                config.max_ttl_secs = val;
            }
        }

        if let Ok(default_ttl) = std::env::var("DENYLIST_DEFAULT_TTL_SECS") {
            if let Ok(val) = default_ttl.parse::<u64>() {
                config.default_ttl_secs = val;
            }
        }

        if let Ok(jitter) = std::env::var("DENYLIST_JITTER_FACTOR") {
            if let Ok(val) = jitter.parse::<f64>() {
                config.jitter_factor = val.clamp(0.0, 1.0);
            }
        }

        if let Ok(redis_url) = std::env::var("DENYLIST_REDIS_URL") {
            config.redis_url = redis_url;
        }

        config
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DenylistConfig::default();
        assert_eq!(config.max_entries, 10_000);
        assert_eq!(config.max_ttl_secs, 300);
        assert_eq!(config.default_ttl_secs, 300);
        assert_eq!(config.jitter_factor, 0.2);
        assert_eq!(config.redis_key_prefix, "denylist");
        assert_eq!(config.redis_url, "redis://127.0.0.1:6379");
    }

    #[test]
    fn test_custom_config() {
        let config = DenylistConfig::new(5_000, 600, 120, 0.3);
        assert_eq!(config.max_entries, 5_000);
        assert_eq!(config.max_ttl_secs, 600);
        assert_eq!(config.default_ttl_secs, 120);
        assert_eq!(config.jitter_factor, 0.3);
    }

    #[test]
    fn test_builder_pattern() {
        let config = DenylistConfig::default()
            .with_max_entries(1_000)
            .with_max_ttl_secs(60)
            .with_default_ttl_secs(30)
            .with_jitter_factor(0.1)
            .with_redis_key_prefix("custom")
            .with_redis_url("redis://localhost:6380");

        assert_eq!(config.max_entries, 1_000);
        assert_eq!(config.max_ttl_secs, 60);
        assert_eq!(config.default_ttl_secs, 30);
        assert_eq!(config.jitter_factor, 0.1);
        assert_eq!(config.redis_key_prefix, "custom");
        assert_eq!(config.redis_url, "redis://localhost:6380");
    }

    #[test]
    fn test_jitter_factor_clamped() {
        // Jitter factor should be clamped to [0.0, 1.0]
        let mut config = DenylistConfig::default();
        config.jitter_factor = -0.5;
        assert_eq!(config.jitter_factor, -0.5); // Not auto-clamped in struct, but validation happens in from_env
        config.jitter_factor = 1.5;
        assert_eq!(config.jitter_factor, 1.5);
        // from_env does clamp
        std::env::set_var("DENYLIST_JITTER_FACTOR", "2.0");
        let config = DenylistConfig::from_env();
        assert_eq!(config.jitter_factor, 1.0);
        std::env::remove_var("DENYLIST_JITTER_FACTOR");
    }
}
