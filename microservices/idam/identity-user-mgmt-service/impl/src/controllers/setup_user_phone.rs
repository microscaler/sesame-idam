use brrtrouter_macros::handler;
use identity_user_mgmt_service_service_api::handlers::setup_user_phone::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(SetupUserPhoneController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::UserManagement,
        "phone_setup",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::User,
        req.inner.ip_address.clone().unwrap_or_else(|| "127.0.0.1".to_string()),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.metadata = serde_json::json!({ "phone": req.inner.phone }).into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    // TODO: UPDATE users SET phone = $1 WHERE id = $2
    // TODO: Send SMS verification code
    
    Response {}
}
