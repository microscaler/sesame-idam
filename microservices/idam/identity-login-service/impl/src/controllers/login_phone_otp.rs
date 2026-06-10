use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::login_phone_otp::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

/// Handler for Login Phone Otp.
#[handler(LoginPhoneOtpController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    // TODO: Look up user by phone number
    // TODO: Generate 6-digit OTP
    // TODO: Store OTP in Redis with 5min TTL
    // TODO: Send OTP via Twilio SMS
    // TODO: Rate limit: max 3 attempts per 5 minutes per phone

    Response { success: Some(true), message: Some("Verification code sent to your phone".to_string()) }
}
