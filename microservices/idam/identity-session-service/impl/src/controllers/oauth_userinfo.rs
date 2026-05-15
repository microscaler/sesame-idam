use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::oauth_userinfo::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(OauthUserinfoController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::SessionManagement,
        "oauth_userinfo_accessed",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::ServiceAccount,
        req.inner.ip_address.clone().unwrap_or_else(|| "127.0.0.1".to_string()),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.metadata = serde_json::json!({ "client_id": req.inner.client_id }).into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    Response {
        success: req.inner.success.unwrap_or(false),
        error: req.inner.error.clone().unwrap_or_default(),
        claims: req.inner.claims.clone().unwrap_or_default(),
    }
}
