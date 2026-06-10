use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::login_email_otp::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

/// Handler for Login Email Otp.
#[handler(LoginEmailOtpController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    // TODO: Look up user by email
    // TODO: If user not found, still return success (prevent email enumeration)
    // TODO: Generate 6-digit OTP
    // TODO: Store OTP in Redis with 5min TTL
    // TODO: Send OTP via SES/SendGrid email
    // TODO: Log login_failure audit event if user doesn't exist

    let mut event = AuditEvent::new(
        AuditEventType::Authentication,
        "email_otp_sent",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::User,
        req.inner.ip_address.clone().unwrap_or_else(|| "127.0.0.1".to_string()),
    );
    event.metadata = serde_json::json!({ "email": req.inner.email }).into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    Response { success: Some(true), message: Some("Verification code sent to your email".to_string()) }
}
