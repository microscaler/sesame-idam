use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_api_keys_gen::handlers::fetch_api_key_usage::{Request, Response};

/// Handler for Fetch Api Key Usage.
#[handler(FetchApiKeyUsageController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditActor, AuditEvent, AuditEventType, AuditSeverity};
    use uuid::Uuid;

    let event = AuditEvent::new_with_params(
        AuditEventType::ApiKey,
        "api_key_usage_accessed",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::User,
        "internal".to_string(),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.metadata =
        serde_json::json!({ "api_key_id": req.inner.api_key_id }).into();
    EMITTER.emit(event);

    Response {
        success: req.inner.success.unwrap_or(false),
        error: req.inner.error.clone().unwrap_or_default(),
    }
}
