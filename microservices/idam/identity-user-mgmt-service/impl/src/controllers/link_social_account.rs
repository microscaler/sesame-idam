use brrtrouter_macros::handler;
use sesame_idam_identity_user_mgmt_service_gen::handlers::link_social_account::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

/// Handler for Link Social Account.
#[handler(LinkSocialAccountController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::UserManagement,
        "social_account_linked",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::User,
        req.inner.ip_address.clone().unwrap_or_else(|| "127.0.0.1".to_string()),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.metadata = serde_json::json!({
        "provider": req.inner.provider,
    }).into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    // TODO: Store social provider user_id in user_social_accounts table
    // TODO: Link to existing user_id
    
    Response {
        social_provider: req.inner.provider,
        success: true,
    }
}
