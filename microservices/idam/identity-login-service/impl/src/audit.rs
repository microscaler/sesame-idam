//! Shared audit logger instance for this service.
//! Services import this as `use crate::audit::EMITTER;`

use sesame_common::audit::AuditEmitter;

/// Global audit emitter shared across all handlers in this service.
#[allow(dead_code)]
pub static EMITTER: std::sync::LazyLock<AuditEmitter> =
    std::sync::LazyLock::new(|| AuditEmitter::new("identity-login-service", None));
