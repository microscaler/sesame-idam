use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::admin_restore_impersonation::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(AdminRestoreImpersonationController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::SessionManagement,
        "impersonation_restored",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.severity = Some(AuditSeverity::Warning);
    EMITTER.emit(&mut event);

    Response { success: req.inner.success.unwrap_or(false) }
}
