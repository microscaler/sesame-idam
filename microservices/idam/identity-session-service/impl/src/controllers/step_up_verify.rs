use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::step_up_verify::{Request, Response};

#[handler(StepUpVerifyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditActor, AuditEvent, AuditEventType, AuditSeverity};
    use uuid::Uuid;

    let tenant_id = _req.data.x_tenant_id.clone();

    let mut event = AuditEvent::new(
        AuditEventType::SessionManagement,
        "step_up_verified",
        tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::User,
        "127.0.0.1".to_string(),
    );
    event.severity = Some(AuditSeverity::Warning);
    EMITTER.emit(&mut event);

    Response {
        mfa_method: None,
        session_id: None,
        verified: false,
    }
}
