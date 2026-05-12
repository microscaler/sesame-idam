use brrtrouter_macros::handler;
use identity_login_service_service_api::handlers::auth_login::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(AuthLoginController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::Authentication,
        if req.inner.success.unwrap_or(false) { "login_success" } else { "login_failure" },
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::User,
        req.inner.ip_address.clone().unwrap_or_else(|| "127.0.0.1".to_string()),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.session_id = req.inner.session_id.parse::<Uuid>().ok();
    event.metadata = serde_json::json!({ "method": req.inner.method }).into();
    event.severity = if req.inner.success.unwrap_or(false) {
        Some(AuditSeverity::Info)
    } else {
        Some(AuditSeverity::Warning)
    };
    EMITTER.emit(&mut event);

    Response {
        error: req.inner.error.clone().unwrap_or_default(),
        success: req.inner.success.unwrap_or(false),
        session_id: req.inner.session_id.clone().unwrap_or_default(),
        user_id: req.inner.user_id.clone().unwrap_or_default(),
    }
}
