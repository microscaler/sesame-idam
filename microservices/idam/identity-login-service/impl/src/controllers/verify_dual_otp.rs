use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::verify_dual_otp::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(VerifyDualOtpController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    // TODO: Verify email OTP from Redis (consume: verify + delete)
    // TODO: Verify phone OTP from Redis (consume: verify + delete)
    // TODO: If both OTPs valid, issue access_token + refresh_token
    // TODO: Mark user as fully verified (both email and phone)
    // TODO: Log login_success audit event

    Response {
        newly_verified_email: Some(true),
        newly_verified_phone: Some(true),
        email_verified: Some(true),
        phone_verified: Some(true),
    }
}
