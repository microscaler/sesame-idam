//! Shared audit logger instance for this service.
//! Services import this as `use crate::audit::EMITTER;`

use sesame_audit::AuditEmitter;

/// Global audit emitter shared across all handlers in this service.
pub static EMITTER: std::sync::LazyLock<AuditEmitter> =
    std::sync::LazyLock::new(|| AuditEmitter::new(None));

/// Global push invalidation publisher for authz state changes.
/// Created during startup from config; `None` if Redis is not configured.
pub static PUBLISHER: std::sync::LazyLock<Option<std::sync::Arc<PublisherWrapper>>> =
    std::sync::LazyLock::new(|| {
        let config = load_config(&std::path::PathBuf::from("./config/config.yaml")).unwrap_or_default();
        push_invalidator::create_publisher(&config).map(|p| std::sync::Arc::new(p))
    });
