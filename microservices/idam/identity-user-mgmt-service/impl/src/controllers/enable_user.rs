use brrtrouter_macros::handler;
use sesame_idam_identity_user_mgmt_service_gen::handlers::enable_user::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

/// Handler for Enable User.
#[handler(EnableUserController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let event = AuditEvent::new_with_params(
        AuditEventType::UserManagement,
        "user_enabled",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    EMITTER.emit(event);

    // TODO: UPDATE users SET disabled = false WHERE id = $1 AND tenant_id = $2
    
    Response {}
}
