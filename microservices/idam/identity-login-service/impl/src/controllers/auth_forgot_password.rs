use brrtrouter_macros::handler;
use identity_login_service_service_api::handlers::auth_forgot_password::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(AuthForgotPasswordController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    // TODO: Look up user by email in the tenant's user table
    // TODO: If user exists, generate reset token, store in Redis with 15min TTL
    // TODO: If user doesn't exist, still return success (prevent email enumeration)
    // TODO: Send email with password reset link containing token

    let mut event = AuditEvent::new(
        AuditEventType::UserManagement,
        "password_reset_requested",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::User,
        req.inner.ip_address.clone().unwrap_or_else(|| "127.0.0.1".to_string()),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.severity = Some(AuditSeverity::Warning);
    EMITTER.emit(&mut event);

    Response { success: true, message: Some("Password reset instructions sent to your email".to_string()) }
}
