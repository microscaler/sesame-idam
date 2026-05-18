use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::admin_restore_impersonation::{
    Request, Response,
};

#[handler(AdminRestoreImpersonationController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditActor, AuditEvent, AuditEventType, AuditSeverity};
    use uuid::Uuid;

    let tenant_id = req.data.x_tenant_id.clone();
    let admin_user_id = req.data.admin_user_id.clone();

    let mut event = AuditEvent::new(
        AuditEventType::SessionManagement,
        "impersonation_restored",
        tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.user_id = admin_user_id.parse::<Uuid>().ok();
    event.severity = Some(AuditSeverity::Warning);
    EMITTER.emit(&mut event);

    Response {
        access_token: "restored-jwt".to_string(),
        impersonated_user_id: admin_user_id.clone(),
        original_user_id: admin_user_id,
        refresh_token: "restored-refresh".to_string(),
    }
}
