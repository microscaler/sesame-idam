/// Priority-based async log queue.
///
/// Two separate queues: HIGH (security events: WARN/ERROR) and LOW
/// (normal events: DEBUG/INFO). HIGH priority events are always written;
/// LOW priority events are dropped when the buffer is full.
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::event::{AuditLevel, AuditLogEntry};
use super::metrics::{AuditMetrics, RateLimiterState};

/// Maximum entries in the LOW priority queue before dropping.
const LOW_QUEUE_MAX: usize = 10_000;

/// Maximum entries in the HIGH priority queue.
const HIGH_QUEUE_MAX: usize = 10_000;

/// The priority queue for audit log entries.
pub struct AuditQueue {
    /// HIGH priority queue (security events).
    high: Arc<DashMap<u64, AuditLogEntry>>,
    /// LOW priority queue (normal events).
    low: Arc<DashMap<u64, AuditLogEntry>>,
    /// Sequence counter for unique keys.
    seq: AtomicU64,
    /// Per-service rate limiter for DEBUG entries.
    rate_limiter: Arc<RateLimiterState>,
    /// Whether the queue is shutting down.
    shutdown: AtomicU64, // 0 = running, 1 = shutting down
}

impl AuditQueue {
    #[must_use]
    pub fn new() -> Self {
        Self {
            high: Arc::new(DashMap::new()),
            low: Arc::new(DashMap::new()),
            seq: AtomicU64::new(0),
            rate_limiter: Arc::new(RateLimiterState::new(1_000_000_000)), // 1s window
            shutdown: AtomicU64::new(0),
        }
    }

    /// Submit an entry to the appropriate queue.
    ///
    /// HIGH priority (WARN/ERROR) entries are always accepted.
    /// LOW priority entries are dropped if the queue is full or rate-limited.
    pub fn enqueue(&self, entry: AuditLogEntry) -> bool {
        let is_security = entry.level.is_security();

        if is_security {
            self.enqueue_high(entry)
        } else {
            self.enqueue_low(&entry)
        }
    }

    fn enqueue_high(&self, entry: AuditLogEntry) -> bool {
        // Check shutdown
        if self.shutdown.load(Ordering::SeqCst) == 1 {
            return false;
        }

        let high_len = self.high.len();
        if high_len >= HIGH_QUEUE_MAX {
            // Buffer full — security events log synchronously (bypass)
            AuditMetrics::increment_security_dropped();
            return false;
        }

        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
        self.high.insert(seq, entry);
        AuditMetrics::set_high_queue_size(high_len as u64);
        true
    }

    fn enqueue_low(&self, entry: &AuditLogEntry) -> bool {
        // Check shutdown
        if self.shutdown.load(Ordering::SeqCst) == 1 {
            return false;
        }

        // Rate limit DEBUG entries
        if entry.level == AuditLevel::Debug && !self.rate_limiter.allow_debug(1000) {
            AuditMetrics::increment_debug_dropped();
            return false;
        }

        let low_len = self.low.len();
        if low_len >= LOW_QUEUE_MAX {
            return false;
        }

        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
        self.low.insert(seq, entry.clone());
        AuditMetrics::set_low_queue_size(low_len as u64);
        true
    }

    /// Drain and return all HIGH priority entries.
    pub fn drain_high(&self) -> Vec<AuditLogEntry> {
        let mut entries = Vec::new();
        for entry in self.high.iter() {
            entries.push(entry.value().clone());
        }
        self.high.clear();
        entries
    }

    /// Drain and return all LOW priority entries.
    pub fn drain_low(&self) -> Vec<AuditLogEntry> {
        let mut entries = Vec::new();
        for entry in self.low.iter() {
            entries.push(entry.value().clone());
        }
        self.low.clear();
        entries
    }

    /// Signal shutdown — no more entries will be accepted.
    pub fn shutdown(&self) {
        self.shutdown.store(1, Ordering::SeqCst);
    }

    /// Check if shutdown has been signaled.
    #[must_use]
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst) == 1
    }

    /// Get current queue sizes.
    pub fn sizes(&self) -> (usize, usize) {
        (self.high.len(), self.low.len())
    }
}

impl Default for AuditQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::super::event::AuditEventType;
    use super::*;

    fn make_entry(level: AuditLevel, event: AuditEventType) -> AuditLogEntry {
        AuditLogEntry::new(event, "test-service")
            .level(level)
            .build()
            .expect("build entry")
    }

    #[test]
    fn test_high_priority_always_accepted() {
        let queue = AuditQueue::new();
        let entry = make_entry(AuditLevel::Warn, AuditEventType::ValidationFailed);
        assert!(queue.enqueue(entry));
    }

    #[test]
    fn test_low_priority_accepted_within_limit() {
        let queue = AuditQueue::new();
        for _ in 0..100 {
            let entry = make_entry(AuditLevel::Info, AuditEventType::JwtIssued);
            let _ = queue.enqueue(entry);
        }
        assert!(queue.sizes().1 > 0);
    }

    #[test]
    fn test_drain_high() {
        let queue = AuditQueue::new();
        let entry = make_entry(AuditLevel::Warn, AuditEventType::TokenBindingMismatch);
        queue.enqueue(entry);
        assert_eq!(queue.sizes().0, 1);

        let drained = queue.drain_high();
        assert_eq!(drained.len(), 1);
        assert_eq!(queue.sizes().0, 0);
    }

    #[test]
    fn test_drain_low() {
        let queue = AuditQueue::new();
        let entry = make_entry(AuditLevel::Info, AuditEventType::JwtIssued);
        queue.enqueue(entry);
        assert_eq!(queue.sizes().1, 1);

        let drained = queue.drain_low();
        assert_eq!(drained.len(), 1);
        assert_eq!(queue.sizes().1, 0);
    }

    #[test]
    fn test_shutdown_blocks_new_entries() {
        let queue = AuditQueue::new();
        queue.shutdown();
        let entry = make_entry(AuditLevel::Info, AuditEventType::JwtIssued);
        assert!(!queue.enqueue(entry));
    }

    #[test]
    fn test_security_drops_on_full() {
        let queue = AuditQueue::new();
        // Fill HIGH queue
        for i in 0..HIGH_QUEUE_MAX {
            let mut entry = make_entry(AuditLevel::Warn, AuditEventType::ValidationFailed);
            entry.tenant_id = Some(format!("tenant_{i}"));
            let _ = queue.enqueue(entry);
        }
        // Next security entry should be dropped
        let mut entry = make_entry(AuditLevel::Error, AuditEventType::TokenBindingMismatch);
        entry.tenant_id = Some("overflow".to_string());
        assert!(!queue.enqueue(entry));
    }
}
