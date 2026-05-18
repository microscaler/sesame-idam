use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::auth_refresh::{Request, Response};

#[handler(AuthRefreshController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    let span = tracing::span!(
        tracing::Level::INFO,
        "token.refreshed",
        user_id = tracing::field::Empty,
        tenant_id = tracing::field::Empty
    );
    let _guard = span.enter();

    use crate::audit::EMITTER;
    use sesame_audit::{AuditActor, AuditEvent, AuditEventType, AuditSeverity};
    use uuid::Uuid;

    let _refresh_token = req.data.refresh_token.clone();
    let tenant_id = req.data.x_tenant_id.clone();

    let mut event = AuditEvent::new(
        AuditEventType::SessionManagement,
        "token_refreshed",
        tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::User,
        "127.0.0.1".to_string(),
    );
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    span.record("tenant_id", &tenant_id);
    span.record("result", "success");

    Response {
        access_token: "refreshed-jwt".to_string(),
        email: None,
        email_verified: None,
        expires_in: 3600,
        id_token: None,
        mfa_required: None,
        phone_verified: None,
        refresh_token: "refreshed-refresh".to_string(),
        refresh_token_expires_in: None,
        scope: None,
        token_type: "Bearer".to_string(),
        user_id: "default".to_string(),
    }
}
