//! # sesame-audit
//!
//! Security audit logging for Sesame-IDAM — structured JSON, priority queues,
//! rate limiting, HMAC signing, metrics.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌──────────────┐     ┌─────────────┐
//! │  Service Code   │────▶│   Emitter    │────▶│  Priority   │
//! │  (emit_*)       │     │  (validate)  │     │  Queue      │
//! └─────────────────┘     └──────────────┘     └─────────────┘
//!                                          │
//!                     ┌────────────────────┼────────────────────┐
//!                     │                    │                    │
//!               ┌─────▼─────┐        ┌─────▼─────┐      ┌─────▼─────┐
//!               │  HIGH Q   │        │   LOW Q   │      │ Rate    │
//!               │ (WARN/ERR)│        │(DEBUG/INFO)│      │ Limiter │
//!               └───────────┘        └───────────┘      └──────────┘
//!                     │                    │
//!                     ▼                    ▼
//!               ┌─────────────────────────────────────┐
//!               │         Flush Task                  │
//!               │  HIGH → tracing::error!             │
//!               │  LOW → tracing::info!               │
//!               └─────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust
//! use sesame_audit::AuditEmitter;
//!
//! // Create emitter for a service
//! let emitter = AuditEmitter::new("identity-login-service", None);
//!
//! // Emit a JWT issued event
//! emitter.emit_jwt_issued(
//!     "user_123",
//!     "tenant_abc",
//!     "profile:read orders:write",
//!     42,
//!     300,
//!     "ES256",
//! );
//! ```
//!
//! ## Security Requirements (HACK-831 through HACK-838)
//!
//! - **HACK-831**: Audit log cannot be suppressed by denylisting jti.
//!   All events are logged regardless of token state. Denylist is NOT checked.
//! - **HACK-832**: Raw JWT strings are NEVER in audit logs. Only metadata.
//! - **HACK-833**: DEBUG logs rate-limited to 1000 entries/sec per service.
//!   Excess entries are dropped. Metrics track drop count.
//! - **HACK-834**: Security events (WARN/ERROR) are logged synchronously.
//!   Normal events (DEBUG/INFO) can be async. Flush on graceful shutdown.
//! - **HACK-835**: All user input is JSON-escaped via serde_json.
//!   Manual string concatenation prohibited.
//! - **HACK-836**: ip_address and user_agent fields available (optional).
//! - **HACK-837**: Event type validated against allowed set before write.
//!   Invalid types rejected and logged as errors.
//! - **HACK-838**: Priority queue with HIGH/LOW split.
//!   HIGH priority events always written; LOW dropped when full.

pub mod emitter;
pub mod event;
pub mod hmac;
pub mod metrics;
pub mod queue;
pub mod rate_limiter;

// Re-export the most commonly used types at crate root
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
