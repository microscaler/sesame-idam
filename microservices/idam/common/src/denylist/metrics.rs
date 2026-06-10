//! Prometheus metrics for the denylist cache.
//!
//! Provides metrics registration and collectors for denylist cache observability.
//!
//! # Metrics
//!
//! - `denylist_cache_size` — Current number of entries in the local cache
//! - `denylist_cache_hits_total` — Total cache hits (JTI found locally)
//! - `denylist_cache_misses_total` — Total cache misses (JTI not in local cache)
//! - `denylist_cache_redis_hits_total` — Total Redis hits (JTI found in Redis)
//! - `denylist_cache_redis_misses_total` — Total Redis misses (JTI not in Redis)
//! - `denylist_cache_redis_errors_total` — Total Redis errors (connection failures)
//! - `denylist_cache_evictions_total` — Total cache evictions (when max entries reached)

use prometheus::{IntCounter, IntGauge, Registry};

/// Prometheus registry namespace for denylist cache metrics.
/// (Used for documentation — metric names are defined inline.)
#[allow(dead_code)]
const DENYLIST_NAMESPACE: &str = "denylist";

/// Registry containing all denylist cache metrics.
pub struct DenylistMetrics {
    /// Current number of entries in the local cache.
    pub cache_size: IntGauge,
    /// Total cache hits — JTI found in local cache (revoked, no Redis call needed).
    pub hits_total: IntCounter,
    /// Total cache misses — JTI not in local cache, Redis was consulted.
    pub misses_total: IntCounter,
    /// Total Redis hits — JTI found in Redis (revoked, added to local cache).
    pub redis_hits_total: IntCounter,
    /// Total Redis misses — JTI not found in Redis (not revoked).
    pub redis_misses_total: IntCounter,
    /// Total Redis errors — connection failures during Redis lookup.
    pub redis_errors_total: IntCounter,
    /// Total cache evictions — entries removed when max entries reached.
    pub evictions_total: IntCounter,
}

impl DenylistMetrics {
    /// Register all denylist cache metrics with a Prometheus registry.
    ///
    /// Returns a `DenylistMetrics` handle that can be used to record metrics
    /// at runtime.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crate::denylist::register_denylist_metrics;
    ///
    /// let registry = prometheus::Registry::new();
    /// let metrics = register_denylist_metrics(&registry).unwrap();
    ///
    /// // Record a cache hit
    /// metrics.hits_total.inc();
    /// ```
    pub fn register(registry: &Registry) -> Result<Self, prometheus::Error> {
        let cache_size = IntGauge::new(
            "denylist_cache_size",
            "Current number of entries in the denylist cache",
        )?;
        registry.register(Box::new(cache_size.clone()))?;

        let hits_total = IntCounter::new(
            "denylist_cache_hits_total",
            "Total denylist cache hits (JTI found locally)",
        )?;
        registry.register(Box::new(hits_total.clone()))?;

        let misses_total = IntCounter::new(
            "denylist_cache_misses_total",
            "Total denylist cache misses (JTI not found locally, Redis consulted)",
        )?;
        registry.register(Box::new(misses_total.clone()))?;

        let redis_hits_total = IntCounter::new(
            "denylist_cache_redis_hits_total",
            "Total Redis hits (JTI found in Redis, added to cache)",
        )?;
        registry.register(Box::new(redis_hits_total.clone()))?;

        let redis_misses_total = IntCounter::new(
            "denylist_cache_redis_misses_total",
            "Total Redis misses (JTI not found in Redis, not revoked)",
        )?;
        registry.register(Box::new(redis_misses_total.clone()))?;

        let redis_errors_total = IntCounter::new(
            "denylist_cache_redis_errors_total",
            "Total Redis connection errors (tokens rejected — fail-closed)",
        )?;
        registry.register(Box::new(redis_errors_total.clone()))?;

        let evictions_total = IntCounter::new(
            "denylist_cache_evictions_total",
            "Total cache evictions (entries removed when max entries reached)",
        )?;
        registry.register(Box::new(evictions_total.clone()))?;

        Ok(Self {
            cache_size,
            hits_total,
            misses_total,
            redis_hits_total,
            redis_misses_total,
            redis_errors_total,
            evictions_total,
        })
    }

    /// Update the cache size metric to the current entry count.
    pub fn set_cache_size(&self, size: usize) {
        self.cache_size.set(size as i64);
    }

    /// Record a cache hit.
    pub fn inc_hits(&self) {
        self.hits_total.inc();
    }

    /// Record a cache miss.
    pub fn inc_misses(&self) {
        self.misses_total.inc();
    }

    /// Record a Redis hit.
    pub fn inc_redis_hits(&self) {
        self.redis_hits_total.inc();
    }

    /// Record a Redis miss.
    pub fn inc_redis_misses(&self) {
        self.redis_misses_total.inc();
    }

    /// Record a Redis error.
    pub fn inc_redis_errors(&self) {
        self.redis_errors_total.inc();
    }

    /// Record a cache eviction.
    pub fn inc_evictions(&self) {
        self.evictions_total.inc();
    }
}

/// Convenience function to register denylist metrics with a registry.
///
/// This is the preferred way to get denylist metrics — it calls
/// `DenylistMetrics::register()` internally.
///
/// # Example
///
/// ```no_run
/// use crate::denylist::register_denylist_metrics;
///
/// let registry = prometheus::Registry::new();
/// let metrics = register_denylist_metrics(&registry).unwrap();
/// ```
pub fn register_denylist_metrics(
    registry: &Registry,
) -> Result<DenylistMetrics, prometheus::Error> {
    DenylistMetrics::register(registry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_metrics() {
        let registry = Registry::new();
        let _metrics = register_denylist_metrics(&registry).unwrap();

        // Verify all metrics are registered
        let metrics_families = registry.gather();
        let metric_names: Vec<_> = metrics_families.iter().map(|mf| mf.get_name()).collect();

        assert!(metric_names.contains(&"denylist_cache_size"));
        assert!(metric_names.contains(&"denylist_cache_hits_total"));
        assert!(metric_names.contains(&"denylist_cache_misses_total"));
        assert!(metric_names.contains(&"denylist_cache_redis_hits_total"));
        assert!(metric_names.contains(&"denylist_cache_redis_misses_total"));
        assert!(metric_names.contains(&"denylist_cache_redis_errors_total"));
        assert!(metric_names.contains(&"denylist_cache_evictions_total"));
    }

    #[test]
    fn test_metric_updates() {
        let registry = Registry::new();
        let metrics = register_denylist_metrics(&registry).unwrap();

        // Update metrics
        metrics.set_cache_size(42);
        metrics.inc_hits();
        metrics.inc_misses();
        metrics.inc_hits();
        metrics.inc_redis_hits();
        metrics.inc_redis_misses();
        metrics.inc_redis_errors();
        metrics.inc_evictions();
        metrics.inc_evictions();
        metrics.inc_evictions();

        // Verify metric values
        let metrics_families = registry.gather();
        let values: std::collections::HashMap<_, _> = metrics_families
            .iter()
            .filter(|mf| mf.get_name().starts_with("denylist_"))
            .filter_map(|mf| {
                let name = mf.get_name().to_string();
                let m = mf.get_metric().first()?;
                let value = match mf.get_field_type() {
                    prometheus::proto::MetricType::GAUGE => m.get_gauge().get_value(),
                    prometheus::proto::MetricType::COUNTER => m.get_counter().get_value(),
                    _ => return None,
                };
                Some((name, value))
            })
            .collect();

        // Check cache_size gauge
        assert_eq!(values.get(&"denylist_cache_size".to_string()), Some(&42.0));
        // Check hits counter
        assert_eq!(
            values.get(&"denylist_cache_hits_total".to_string()),
            Some(&2.0)
        );
        // Check misses counter
        assert_eq!(
            values.get(&"denylist_cache_misses_total".to_string()),
            Some(&1.0)
        );
        // Check evictions counter
        assert_eq!(
            values.get(&"denylist_cache_evictions_total".to_string()),
            Some(&3.0)
        );
    }

    #[test]
    fn test_empty_cache_size() {
        let registry = Registry::new();
        let _metrics = register_denylist_metrics(&registry).unwrap();

        // Default cache size should be 0
        let metrics_families = registry.gather();
        let cache_size = metrics_families
            .iter()
            .filter(|mf| mf.get_name() == "denylist_cache_size")
            .filter_map(|mf| {
                let m = mf.get_metric().first()?;
                Some(m.get_gauge().get_value())
            })
            .next();

        assert_eq!(cache_size, Some(0.0));
    }
}
