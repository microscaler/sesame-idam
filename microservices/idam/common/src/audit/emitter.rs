/// Audit event emitter — the main interface for producing audit log entries.
///
/// The emitter handles:
/// - Building entries via the builder pattern
/// - Rate limiting (DEBUG at 1000/sec, security events unlimited)
/// - Priority queuing (HIGH for security, LOW for normal)
/// - HMAC signing (optional)
/// - Validation and sanitization before enqueueing
/// - Async dispatch to flush task
/// - Graceful shutdown with buffer flush
use std::sync::Arc;

use super::event::{AuditEventType, AuditLevel, AuditLogEntry};
use super::hmac::sign_entry;
use super::metrics::AuditMetrics;
use super::queue::AuditQueue;
use super::rate_limiter::{RateLimitConfig, RateLimiter};

/// The audit event emitter.
///
/// This is the main entry point for all audit logging. It is thread-safe
/// and can be shared across tasks via `Arc`.
pub struct AuditEmitter {
    service: String,
    queue: Arc<AuditQueue>,
    rate_limiter: Arc<RateLimiter>,
    hmac_key: Option<Vec<u8>>,

}

impl AuditEmitter {
    /// Create a new audit emitter for the given service.
    ///
    /// `hmac_key` — if Some, all entries will be signed with HMAC-SHA256.
    /// Pass None to disable signing.
    pub fn new(service: impl Into<String>, hmac_key: Option<Vec<u8>>) -> Self {
        Self {
            service: service.into(),
            queue: Arc::new(AuditQueue::new()),
            rate_limiter: Arc::new(RateLimiter::new(RateLimitConfig::default())),
            hmac_key,
// shutdown: removed — Notify was dead code (never awaited),
        }
    }

    /// Create with custom rate limit config.
    pub fn new_with_config(
        service: impl Into<String>,
        hmac_key: Option<Vec<u8>>,
        config: RateLimitConfig,
    ) -> Self {
        Self {
            service: service.into(),
            queue: Arc::new(AuditQueue::new()),
            rate_limiter: Arc::new(RateLimiter::new(config)),
            hmac_key,
// shutdown: removed — Notify was dead code (never awaited),
        }
    }

    /// Emit a JWT issued event.
    ///
    /// This is an INFO-level event logged during normal operation.
    pub fn emit_jwt_issued(
        &self,
        user_id: impl Into<String>,
        tenant_id: impl Into<String>,
        scopes: impl Into<String>,
        token_version: u64,
        ttl: u64,
        algorithm: impl Into<String>,
    ) {
        let entry = AuditLogEntry::new(AuditEventType::JwtIssued, &self.service)
            .user_id(user_id)
            .tenant_id(tenant_id)
            .scopes(scopes)
            .token_version(token_version)
            .ttl(ttl)
            .algorithm(algorithm)
            .decision_source("jwt_claims")
            .result("allowed")
            .build();

        if let Ok(entry) = entry {
            self.emit(entry);
        }
    }

    /// Emit a JWT validated event (success).
    ///
    /// This is a DEBUG-level event — rate limited at 1000/sec.
    pub fn emit_jwt_validated(
        &self,
        user_id: impl Into<String>,
        tenant_id: impl Into<String>,
        scopes: impl Into<String>,
        decision_source: impl Into<String>,
    ) {
        let entry = AuditLogEntry::new(AuditEventType::JwtValidated, &self.service)
            .user_id(user_id)
            .tenant_id(tenant_id)
            .scopes(scopes)
            .decision_source(decision_source)
            .result("allowed")
            .build();

        if let Ok(entry) = entry {
            self.emit(entry);
        }
    }

    /// Emit a validation failed event.
    ///
    /// This is a WARN-level security event — always logged synchronously.
    pub fn emit_validation_failed(
        &self,
        user_id: impl Into<String>,
        tenant_id: impl Into<String>,
        scopes: impl Into<String>,
        error: impl Into<String>,
        reason: impl Into<String>,
    ) {
        let entry = AuditLogEntry::new(AuditEventType::ValidationFailed, &self.service)
            .user_id(user_id)
            .tenant_id(tenant_id)
            .scopes(scopes)
            .decision_source("jwt_claims")
            .result("denied")
            .error(error)
            .reason(reason)
            .build();

        if let Ok(entry) = entry {
            self.emit(entry);
        }
    }

    /// Emit a token revoked event.
    ///
    /// This is a WARN-level security event — always logged synchronously.
    pub fn emit_token_revoked(
        &self,
        user_id: impl Into<String>,
        tenant_id: impl Into<String>,
        jti: impl Into<String>,
        reason: impl Into<String>,
    ) {
        let mut entry = AuditLogEntry::new(AuditEventType::TokenRevoked, &self.service)
            .user_id(user_id)
            .tenant_id(tenant_id)
            .decision_source("denylist")
            .result("revoked")
            .reason(reason)
            .build();

        // Add JTI to metadata
        if let Ok(ref mut entry) = entry {
            entry.metadata = Some(serde_json::json!({ "jti": jti.into() }));
            let built = entry.clone();
            self.emit(built);
        }
    }

    /// Emit a family revoked event.
    ///
    /// This is a WARN-level security event — always logged synchronously.
    pub fn emit_family_revoked(
        &self,
        user_id: impl Into<String>,
        tenant_id: impl Into<String>,
        scope: impl Into<String>,
    ) {
        let entry = AuditLogEntry::new(AuditEventType::FamilyRevoked, &self.service)
            .user_id(user_id)
            .tenant_id(tenant_id)
            .decision_source("family_revoke")
            .result("revoked")
            .reason(format!("Token family revoked: {}", scope.into()))
            .build();

        if let Ok(entry) = entry {
            self.emit(entry);
        }
    }

    /// Emit a delegation event.
    ///
    /// This is an INFO-level event logged during delegation/impersonation.
    pub fn emit_delegation(
        &self,
        user_id: impl Into<String>,
        actor_id: impl Into<String>,
        tenant_id: impl Into<String>,
        scopes: impl Into<String>,
        delegation_type: impl Into<String>,
        actor_roles: Vec<String>,
        act_claim_present: bool,
    ) {
        let entry = AuditLogEntry::new(AuditEventType::Delegation, &self.service)
            .user_id(user_id)
            .actor_id(actor_id)
            .tenant_id(tenant_id)
            .scopes(scopes)
            .decision_source("jwt_claims")
            .result("allowed")
            .delegation_type(delegation_type)
            .actor_roles(actor_roles)
            .act_claim_present(act_claim_present)
            .build();

        if let Ok(entry) = entry {
            self.emit(entry);
        }
    }

    /// Emit a version bump event.
    ///
    /// This is an INFO-level event logged during token version management.
    pub fn emit_version_bump(
        &self,
        user_id: impl Into<String>,
        tenant_id: impl Into<String>,
        old_ver: u64,
        new_ver: u64,
        reason: impl Into<String>,
    ) {
        let entry = AuditLogEntry::new(AuditEventType::VersionBump, &self.service)
            .user_id(user_id)
            .tenant_id(tenant_id)
            .decision_source("authz_core")
            .result("allowed")
            .old_ver(old_ver)
            .new_ver(new_ver)
            .version_reason(reason)
            .build();

        if let Ok(entry) = entry {
            self.emit(entry);
        }
    }

    /// Emit a version mismatch event.
    ///
    /// This is a WARN-level security event — logged when a token has a stale version.
    pub fn emit_version_mismatch(
        &self,
        user_id: impl Into<String>,
        tenant_id: impl Into<String>,
        token_ver: u64,
        cached_ver: u64,
    ) {
        let entry = AuditLogEntry::new(AuditEventType::VersionMismatch, &self.service)
            .user_id(user_id)
            .tenant_id(tenant_id)
            .decision_source("jwt_claims")
            .result("denied")
            .error("stale_auth_token")
            .reason(format!(
                "claims.ver ({}) < cached_ver ({})",
                token_ver, cached_ver
            ))
            .build();

        if let Ok(entry) = entry {
            self.emit(entry);
        }
    }

    /// Emit a token binding mismatch event.
    ///
    /// This is an ERROR-level event — active attack indicator.
    pub fn emit_binding_mismatch(
        &self,
        user_id: impl Into<String>,
        tenant_id: impl Into<String>,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) {
        let entry = AuditLogEntry::new(AuditEventType::TokenBindingMismatch, &self.service)
            .user_id(user_id)
            .tenant_id(tenant_id)
            .decision_source("dpop_verify")
            .result("denied")
            .expected_binding(expected)
            .actual_binding(actual)
            .build();

        if let Ok(entry) = entry {
            self.emit(entry);
        }
    }

    /// Core emit method — rate limiting, validation, queueing, and HMAC signing.
    pub fn emit(&self, mut entry: AuditLogEntry) {
        // HACK-835: Rate limit DEBUG events (HACK-833)
        if entry.level == AuditLevel::Debug {
            if !self.rate_limiter.allow_debug(&self.service) {
                // Dropped — don't enqueue
                return;
            }
        }
        // Security events (WARN/ERROR) are always allowed

        // HACK-832: Validate no raw JWT tokens in entry
        if let Err(e) = entry.validate() {
            tracing::error!(error = %e, "audit_log_entry_dropped");
            return;
        }

        // Sanitize fields (truncate long strings)
        entry.sanitize();

        // Sign if HMAC key is configured
        if let Some(ref key) = self.hmac_key {
            if let Ok(log_json) = entry.to_log_json() {
                let timestamp = entry.timestamp.to_rfc3339();
                entry.hmac_signature = Some(sign_entry(key, &log_json, &timestamp));
            }
        }

        // Record metric
        AuditMetrics::increment_total(&entry.event);

        // Enqueue to priority queue
        let _ = self.queue.enqueue(entry);

        // Signal the flush task if there's new work
    }

    /// Flush all pending entries synchronously.
    ///
    /// Returns the number of entries flushed.
    pub fn flush(&self) -> usize {
        let mut count = 0;

        // Drain HIGH priority first (security events)
        let high_entries = self.queue.drain_high();
        count += high_entries.len();
        for entry in high_entries {
            if let Ok(json) = entry.to_log_json() {
                tracing::error!(log = json, "audit_security_event");
            }
        }

        // Drain LOW priority
        let low_entries = self.queue.drain_low();
        count += low_entries.len();
        for entry in low_entries {
            if let Ok(json) = entry.to_log_json() {
                tracing::info!(log = json, "audit_event");
            }
        }

        count
    }

    /// Shutdown the emitter and flush all pending entries.
    pub fn shutdown(&self) {
        self.queue.shutdown();
        self.flush();
    }

    /// Get current queue sizes.
    #[must_use]
    pub fn queue_sizes(&self) -> (usize, usize) {
        self.queue.sizes()
    }
}

impl Clone for AuditEmitter {
    fn clone(&self) -> Self {
        Self {
            service: self.service.clone(),
            queue: Arc::clone(&self.queue),
            rate_limiter: Arc::clone(&self.rate_limiter),
            hmac_key: self.hmac_key.clone(),
// shutdown: removed — Notify was dead code (never awaited),
        }
    }
}