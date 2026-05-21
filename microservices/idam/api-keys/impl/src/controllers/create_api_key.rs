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
    use sesame_audit::{AuditActor, AuditEvent, AuditEventType, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::ApiKey,
        "api_key_created",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::ApiKey,
        "internal".to_string(),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.metadata =
        serde_json::json!({ "key_name": req.inner.name, "permissions": req.inner.permissions }).into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    Response {
        success: req.inner.success.unwrap_or(false),
        error: req.inner.error.clone().unwrap_or_default(),
        api_key: req.inner.api_key.clone().unwrap_or_default(),
    }
}
