use brrtrouter_macros::handler;
use sesame_idam_api_keys_gen::handlers::validate_api_key::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(ValidateApiKeyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::ApiKey,
        "api_key_validated",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::ApiKey,
        "internal".to_string(),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.metadata = serde_json::json!({ "valid": req.inner.valid, "key_prefix": req.inner.key_prefix }).into();
    event.severity = if req.inner.valid.unwrap_or(false) {
        Some(AuditSeverity::Info)
    } else {
        Some(AuditSeverity::Warning)
    };
    EMITTER.emit(&mut event);

    Response {
        valid: req.inner.valid.unwrap_or(false),
        error: req.inner.error.clone().unwrap_or_default(),
    }
}
