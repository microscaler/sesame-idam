pub mod emitter;
pub mod event;
pub mod hmac;
pub mod metrics;
pub mod queue;
pub mod rate_limiter;

// Re-export the most commonly used types at module root
pub use emitter::AuditEmitter;
pub use event::{
    allowed_event_types, is_valid_event_type, AuditEventType, AuditLevel, AuditLogEntry,
    AuditLogEntryBuilder,
};
pub use hmac::{generate_key, sign_entry, verify_entry};
pub use metrics::AuditMetrics;
pub use queue::AuditQueue;
pub use rate_limiter::{RateLimitConfig, RateLimiter};

// AuditEvent and AuditSeverity compatibility aliases (used by existing consumers)
pub type AuditEvent = AuditLogEntry;
pub type AuditSeverity = AuditLevel;

// AuditActor — simplified actor type used by auth controller
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditActor {
    User,
    Admin,
    System,
}

impl AuditActor {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditActor::User => "user",
            AuditActor::Admin => "admin",
            AuditActor::System => "system",
        }
    }
}
