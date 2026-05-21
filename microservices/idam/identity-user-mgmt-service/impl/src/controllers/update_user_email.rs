use brrtrouter_macros::handler;
use sesame_idam_identity_user_mgmt_service_gen::handlers::update_user_email::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

/// Handler for Update User Email.
#[handler(UpdateUserEmailController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let event = AuditEvent::new_with_params(
        AuditEventType::UserManagement,
        "email_updated",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::User,
        req.inner.ip_address.clone().unwrap_or_else(|| "127.0.0.1".to_string()),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.metadata = serde_json::json!({
        "new_email": req.inner.email,
    }).into();
    EMITTER.emit(event);

    // TODO: UPDATE users SET email = $1, email_verified = false WHERE id = $2
    // TODO: Send confirmation email to new address
    
    Response {}
}
