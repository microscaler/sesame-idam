/// Rate limiting for audit log entries.
///
/// Implements:
/// - DEBUG rate limit: max 1000 entries/sec per service (HACK-833)
/// - INFO/WARN/ERROR: no rate limiting (always logged)
/// - Metrics for dropped entries

use std::sync::Arc;
use std::time::Instant;

/// Rate limiter configuration.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Max DEBUG entries per second per service.
    pub max_debug_per_second: u64,
    /// Window size in nanoseconds.
    pub window_ns: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_debug_per_second: 1000,
            window_ns: 1_000_000_000, // 1 second
        }
    }
}

/// Per-service rate limiter state.
pub struct RateLimiter {
    config: RateLimitConfig,
    /// Per-service debug counters. Keyed by service name.
    counters: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, CounterState>>>,
}

struct CounterState {
    count: u64,
    window_start: Instant,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            counters: std::sync::Arc::new(std::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
        }
    }

    /// Check if a DEBUG-level entry should be allowed.
    pub fn allow_debug(&self, service: &str) -> bool {
        let now = Instant::now();
        let mut map = self.counters.lock().unwrap();

        // Get or create counter for this service
        let state = map
            .entry(service.to_string())
            .or_insert_with(|| CounterState {
                count: 0,
                window_start: now,
            });

        // Check if we've moved to a new window
        if now.duration_since(state.window_start).as_nanos() as u64 >= self.config.window_ns {
            state.count = 1;
            state.window_start = now;
            return true;
        }

        // Check limit
        if state.count >= self.config.max_debug_per_second {
            crate::metrics::AuditMetrics::increment_debug_dropped();
            return false;
        }

        state.count += 1;
        true
    }

    /// Security events (WARN/ERROR) are always allowed.
    #[must_use]
    pub fn allow_security(&self) -> bool {
        true
    }

    /// Get the current drop count for debugging.
    #[must_use]
    pub fn debug_drops(&self) -> u64 {
        // TODO: expose via metrics registry
        0
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(RateLimitConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_debug_per_second: 100,
            ..Default::default()
        });

        for _ in 0..100 {
            assert!(limiter.allow_debug("test-service"));
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_debug_per_second: 100,
            ..Default::default()
        });

        for _ in 0..100 {
            let _ = limiter.allow_debug("test-service");
        }
        // Next should be denied
        assert!(!limiter.allow_debug("test-service"));
    }

    #[test]
    fn test_per_service_limits() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_debug_per_second: 10,
            ..Default::default()
        });

        // Exhaust service A
        for _ in 0..10 {
            let _ = limiter.allow_debug("service-a");
        }
        assert!(!limiter.allow_debug("service-a"));

        // Service B should still have capacity
        assert!(limiter.allow_debug("service-b"));
    }

    #[test]
    fn test_security_always_allowed() {
        let limiter = RateLimiter::new(RateLimitConfig::default());
        for _ in 0..10000 {
            assert!(limiter.allow_security());
        }
    }
}
