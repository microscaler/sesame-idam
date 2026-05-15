use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::users_me_patch::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(UsersMePatchController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::UserManagement,
        "user_profile_updated",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::User,
        req.inner.ip_address.clone().unwrap_or_else(|| "127.0.0.1".to_string()),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    Response {
        error: req.inner.error.clone().unwrap_or_default(),
        success: req.inner.success.unwrap_or(false),
    }
}
