use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::step_up_verify::{Request, Response};

#[handler(StepUpVerifyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditActor, AuditEvent, AuditEventType, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::SessionManagement,
        "step_up_verified",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::User,
        req.inner
            .ip_address
            .clone()
            .unwrap_or_else(|| "127.0.0.1".to_string()),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.session_id = req.inner.session_id.parse::<Uuid>().ok();
    event.severity = Some(AuditSeverity::Warning);
    EMITTER.emit(&mut event);

    Response {
        success: req.inner.success.unwrap_or(false),
    }
}
