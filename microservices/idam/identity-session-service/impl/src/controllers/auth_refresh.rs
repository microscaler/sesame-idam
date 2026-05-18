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

    let user_id = req.inner.user_id.clone();
    let tenant_id = req.inner.tenant_id.clone();

    let mut event = AuditEvent::new(
        AuditEventType::SessionManagement,
        "token_refreshed",
        tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::User,
        req.inner
            .ip_address
            .clone()
            .unwrap_or_else(|| "127.0.0.1".to_string()),
    );
    event.user_id = user_id.parse::<Uuid>().ok();
    event.session_id = req.inner.session_id.parse::<Uuid>().ok();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    span.record("user_id", &user_id);
    span.record("tenant_id", &tenant_id);
    span.record("result", if req.inner.success.unwrap_or(false) { "success" } else { "denied" });

    Response {
        success: req.inner.success.unwrap_or(false),
        error: req.inner.error.clone().unwrap_or_default(),
        access_token: req.inner.access_token.clone().unwrap_or_default(),
        refresh_token: req.inner.refresh_token.clone().unwrap_or_default(),
    }
}
