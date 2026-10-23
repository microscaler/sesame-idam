use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_api_keys_gen::handlers::delete_api_key::{Request, Response};

/// Handler for Delete Api Key.
#[handler(DeleteApiKeyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    // Span: api_key.deleted
    let _span = tracing::span!(
        tracing::Level::INFO,
        "api_key.deleted",
        tenant_id = tracing::field::Empty,
        result = tracing::field::Empty
    );
    // Not entered (may_tracing ADR-0001): a may coroutine can resume on another thread after a yield, and a span
    // entered on one thread and exited on another leaks its context there for good (2026-10-23 memory incident).
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEventType, AuditLevel, AuditLogEntry};
    use uuid::Uuid;

    let entry = AuditLogEntry::new(AuditEventType::Delegation, "api-keys")
        .tenant_id(req.inner.tenant_id.clone())
        .metadata(serde_json::json!({
            "api_key_id": req.inner.api_key_id,
        }))
        .build();

    let entry = entry.and_then(|e| {
        Ok(e.user_id(
            req.inner.user_id
                .parse::<Uuid>()
                .ok()
                .map(|u| u.to_string())
                .unwrap_or_default(),
        )
        .level(AuditLevel::Warn)
        .build()?)
    });

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    Response {
        success: req.inner.success.unwrap_or(false),
        error: req.inner.error.clone().unwrap_or_default(),
    }
}
