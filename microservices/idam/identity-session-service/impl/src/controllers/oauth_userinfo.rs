/// Handler for OAuth Userinfo — returns OAuth user information..
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::oauth_userinfo::{Request, Response};

#[handler(OauthUserinfoController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditActor, AuditEvent, AuditEventType, AuditSeverity};
    use uuid::Uuid;

    let tenant_id = _req.data.x_tenant_id.clone();

    let mut event = AuditEvent::new_with_params(
        AuditEventType::SessionManagement,
        "oauth_userinfo_accessed",
        tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::ServiceAccount,
        "127.0.0.1".to_string(),
    );
    EMITTER.emit(event);

    Response {
        email: None,
        email_verified: None,
        first_name: None,
        last_name: None,
        name: None,
        org_id: None,
        org_name: None,
        phone_number: None,
        phone_verified: None,
        picture_url: None,
        preferred_username: None,
        properties: None,
        sub: None,
        updated_at: None,
        user_id: None,
        user_permissions: None,
        user_role: None,
        username: None,
    }
}
