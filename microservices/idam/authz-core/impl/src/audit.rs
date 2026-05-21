//! Shared audit logger and push invalidation publisher for this service.
//!
//! Services import the EMITTER as `use crate::audit::EMITTER;`.
//! The PUBLISHER is created at startup from config and is `None` if Redis is
//! not configured. Controllers call it after authz state changes.

use sesame_audit::AuditEmitter;

/// Global audit emitter shared across all handlers in this service.
pub static EMITTER: std::sync::LazyLock<AuditEmitter> =
    std::sync::LazyLock::new(|| AuditEmitter::new(None));

/// Global push invalidation publisher for authz state changes.
/// Created during startup from config; `None` if Redis is not configured.
///
/// Controllers use it like:
/// ```
/// use crate::audit::PUBLISHER;
/// if let Some(pub_) = &*PUBLISHER {
///     pub_.publish_tenant(&tenant_id, new_ver, BumpReason::RoleRevoked);
/// }
/// ```
pub static PUBLISHER: std::sync::LazyLock<
    Option<std::sync::Arc<crate::push_invalidator::PublisherWrapper>>,
> = std::sync::LazyLock::new(|| {
    let config = crate::config::load_config(&std::path::PathBuf::from("./config/config.yaml"))
        .unwrap_or_default();
    crate::push_invalidator::create_publisher(&config).map(|p| std::sync::Arc::new(p))
});
