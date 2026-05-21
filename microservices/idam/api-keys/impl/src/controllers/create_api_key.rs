use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_api_keys_gen::handlers::create_api_key::{Request, Response};

/// Handler for Create Api Key.
#[handler(CreateApiKeyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    // Span: api_key.created
    let span = tracing::span!(
        tracing::Level::INFO,
        "api_key.created",
        tenant_id = tracing::field::Empty,
        result = tracing::field::Empty
    );
    let _guard = span.enter();
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEventType, AuditLevel, AuditLogEntry};
    use uuid::Uuid;

    let entry = AuditLogEntry::new(AuditEventType::Delegation, "api-keys")
        .tenant_id(req.inner.tenant_id.clone())
        .metadata(serde_json::json!({
            "key_name": req.inner.name,
            "permissions": req.inner.permissions,
            "user_id": req.inner.user_id,
        }))
        .build();

    if let Ok(entry) = entry {
        if let Ok(uid) = req.inner.user_id.parse::<Uuid>() {
            EMITTER.emit(entry.user_id(uid.to_string()));
        } else {
            EMITTER.emit(entry);
        }
    }

    Response {
        success: req.inner.success.unwrap_or(false),
        error: req.inner.error.clone().unwrap_or_default(),
        api_key: req.inner.api_key.clone().unwrap_or_default(),
    }
}
