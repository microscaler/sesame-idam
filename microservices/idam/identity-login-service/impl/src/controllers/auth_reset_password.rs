use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::auth_reset_password::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(AuthResetPasswordController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    // TODO: Validate reset token from Redis (check TTL, verify it hasn't been used)
    // TODO: Hash new password with bcrypt/argon2
    // TODO: UPDATE users SET password_hash = $1
    // TODO: Delete reset token from Redis (one-time use)
    // TODO: Invalidate all existing sessions for this user
    // TODO: Send "password changed" notification email

    Response { success: true, message: Some("Password has been reset successfully".to_string()) }
}
