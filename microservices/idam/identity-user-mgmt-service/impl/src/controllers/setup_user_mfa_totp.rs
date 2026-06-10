use brrtrouter_macros::handler;
use sesame_idam_identity_user_mgmt_service_gen::handlers::setup_user_mfa_totp::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

/// Handler for Setup User Mfa Totp.
#[handler(SetupUserMfaTotpController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::UserManagement,
        "mfa_enrolled",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::User,
        req.inner.ip_address.clone().unwrap_or_else(|| "127.0.0.1".to_string()),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    // TODO: Generate TOTP secret (RFC 4226)
    // TODO: Store encrypted secret in user_mfa_devices table
    // TODO: Return QR code URI for the user to scan with authenticator app
    
    Response {
        provisioning_uri: format!(
            "otpauth://totp/{}:{}",
            "sesame-idam",
            req.inner.user_id
        ),
        secret: Some("TOTP_SECRET_PLACEHOLDER".to_string()),
    }
}
