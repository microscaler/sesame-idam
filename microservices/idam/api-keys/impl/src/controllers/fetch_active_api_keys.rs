use brrtrouter_macros::handler;
use sesame_idam_api_keys_gen::handlers::fetch_active_api_keys::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(FetchActiveApiKeysController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::ApiKey,
        "api_keys_listed",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::User,
        "internal".to_string(),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    Response {
        success: req.inner.success.unwrap_or(false),
        error: req.inner.error.clone().unwrap_or_default(),
        keys: Vec::new(),
    }
}
