use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::social_login::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

/// Handler for Social Login.
#[handler(SocialLoginController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let event = AuditEvent::new_with_params(
        AuditEventType::Authentication,
        "social_login_success",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::User,
        req.inner.ip_address.clone().unwrap_or_else(|| "127.0.0.1".to_string()),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.session_id = req.inner.session_id.parse::<Uuid>().ok();
    event.metadata = serde_json::json!({ "provider": req.inner.provider }).into();
    EMITTER.emit(event);

    Response {
        success: req.inner.success.unwrap_or(false),
        error: req.inner.error.clone().unwrap_or_default(),
        session_id: req.inner.session_id.clone().unwrap_or_default(),
        user_id: req.inner.user_id.clone().unwrap_or_default(),
    }
}
