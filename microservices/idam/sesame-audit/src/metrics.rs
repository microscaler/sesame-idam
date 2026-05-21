//! Metrics for the audit logging system.
//!
//! Tracks:
//! - Total events emitted per event type
//! - Token version counters (issued, bumped)
//! - Drops due to rate limiting
//! - Buffer sizes and latency

use metrics::{counter, describe_counter, gauge};
use std::sync::atomic::{AtomicU64, Ordering};

/// Event counter metric type.
pub struct AuditMetrics;

impl AuditMetrics {
    /// Total audit log events emitted, partitioned by event type.
    pub fn increment_total(event_type: &str) {
        counter!("audit_log_events_total", "event" => event_type.to_string()).increment(1);
    }

    /// Increment debug-level drop count when rate-limited.
    pub fn increment_debug_dropped() {
        counter!("audit_log_debug_dropped_total").increment(1);
    }

    /// Increment WARN/ERROR drop count when buffer is full.
    pub fn increment_security_dropped() {
        counter!("audit_log_security_dropped_total").increment(1);
    }

    /// Set the current queue size for the HIGH priority queue.
    pub fn set_high_queue_size(size: u64) {
        gauge!("audit_log_queue_size_high").set(size as f64);
    }

    /// Set the current queue size for the LOW priority queue.
    pub fn set_low_queue_size(size: u64) {
        gauge!("audit_log_queue_size_low").set(size as f64);
    }

    /// Record log write latency in seconds.
    pub fn record_latency(duration: std::time::Duration) {
        // Metrics 0.22 uses gauge for timing (counter only supports u64)
        let sec = duration.as_secs_f64();
        gauge!("audit_log_latency_seconds").set(sec);
    }

    /// Increment counter for tokens issued (jwt_issued events).
    pub fn increment_token_issued() {
        counter!("token_version_total", "action" => "issued").increment(1);
    }

    /// Increment counter for token version bumps.
    pub fn increment_token_bumped() {
        counter!("token_version_total", "action" => "bumped").increment(1);
    }

    /// Set the current token version for a subject.
    pub fn set_current_token_version(subject_id: &str, version: u64) {
        gauge!("token_version_current", "subject" => subject_id.to_string()).set(version as f64);
    }

    /// Initialize all metric descriptions.
    pub fn init() {
        describe_counter!(
            "audit_log_events_total",
            "Total audit log events emitted, partitioned by event type"
        );
        describe_counter!(
            "audit_log_debug_dropped_total",
            "Total DEBUG-level log entries dropped due to rate limiting"
        );
        describe_counter!(
            "audit_log_security_dropped_total",
            "Total security events dropped due to buffer overflow"
        );
        describe_counter!(
            "audit_log_queue_size_high",
            "Current size of HIGH priority (security) log queue"
        );
        describe_counter!(
            "audit_log_queue_size_low",
            "Current size of LOW priority (normal) log queue"
        );
    }
}

/// Per-service rate limiter state.
///
/// Thread-safe counter for tracking events per second per service.
pub struct RateLimiterState {
    /// Count of debug entries emitted in the current window.
    debug_count: AtomicU64,
    /// Count of security entries emitted in the current window.
    security_count: AtomicU64,
    /// Start of the current rate-limiting window.
    window_start: std::sync::atomic::AtomicU64,
    /// Window size in nanoseconds (default: 1 second).
    window_ns: u64,
}

impl RateLimiterState {
    /// Create a new rate limiter state with a 1-second window.
    pub const fn new(window_ns: u64) -> Self {
        Self {
            debug_count: AtomicU64::new(0),
            security_count: AtomicU64::new(0),
            window_start: std::sync::atomic::AtomicU64::new(0),
            window_ns,
        }
    }

    /// Check if a DEBUG-level entry is allowed.
    ///
    /// Returns true if the entry should be written, false if it should be dropped.
    pub fn allow_debug(&self, limit: u64) -> bool {
        self.check_count(&self.debug_count, limit)
    }

    /// Check if a security-level entry is allowed.
    ///
    /// Security events are always allowed (they bypass rate limiting).
    #[must_use]
    pub fn allow_security(&self) -> bool {
        true
    }

    fn check_count(&self, counter: &AtomicU64, limit: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let current_window = self
            .window_start
            .load(Ordering::Relaxed)
            .max(1); // avoid div by zero

        // If we've moved to a new window, reset counter
        let this_window = (now / self.window_ns) * self.window_ns;
        if this_window != current_window {
            self.window_start.store(this_window, Ordering::Relaxed);
            counter.store(1, Ordering::SeqCst);
            return true;
        }

        let count = counter.load(Ordering::SeqCst);
        if count >= limit {
            return false;
        }

        // Atomically increment and check
        counter.compare_exchange(
            count,
            count + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        )
        .is_ok()
    }
}

impl Default for RateLimiterState {
    fn default() -> Self {
        Self::new(1_000_000_000) // 1 second in nanoseconds
    }
}

// ─── Benchmarks ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_initial_state() {
        let state = RateLimiterState::new(1_000_000_000);
        assert!(state.allow_debug(100));
        assert!(state.allow_security());
    }

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let state = RateLimiterState::new(1_000_000_000);
        for _ in 0..99 {
            assert!(state.allow_debug(100));
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let state = RateLimiterState::new(1_000_000_000);
        // Exhaust the limit
        for _ in 0..100 {
            let _ = state.allow_debug(100);
        }
        // Next should be denied (compare_exchange will fail)
        assert!(!state.allow_debug(100));
    }

    #[test]
    fn test_security_events_always_allowed() {
        let state = RateLimiterState::new(1_000_000_000);
        for _ in 0..1000 {
            assert!(state.allow_security());
        }
    }
}
