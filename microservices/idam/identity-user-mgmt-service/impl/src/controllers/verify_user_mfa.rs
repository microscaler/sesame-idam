use brrtrouter_macros::handler;
use sesame_idam_identity_user_mgmt_service_gen::handlers::verify_user_mfa::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

/// Handler for Verify User Mfa.
#[handler(VerifyUserMfaController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::UserManagement,
        "mfa_verified",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::User,
        req.inner.ip_address.clone().unwrap_or_else(|| "127.0.0.1".to_string()),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    // TODO: Verify TOTP code against stored secret (RFC 4226)
    // TODO: Issue new JWT with MFA verified claim
    
    Response {
        user_id: req.inner.user_id,
        mfa_verified: true,
    }
}
