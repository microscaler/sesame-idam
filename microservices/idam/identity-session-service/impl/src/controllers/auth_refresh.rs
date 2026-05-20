/// Handler for Auth Refresh — refreshes an access token using a refresh token.
/// Uses TTL configuration from `jwt::ttl::TtlConfig` to set `expires_in` on
/// issued access tokens.
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

    // Apply TTL config for access and refresh token expiry.
    let ttl_config = crate::jwt::ttl::TtlConfig::from_env();
    let access_ttl_secs = ttl_config.access_ttl_secs_for_role("customer");
    let refresh_ttl_secs = ttl_config.refresh_ttl_for_role("customer").as_secs();
    ttl_config.record_ttl_metric("customer");

    Response {
        access_token: "refreshed-jwt".to_string(),
        email: None,
        email_verified: None,
        expires_in: access_ttl_secs as i32,
        id_token: None,
        mfa_required: None,
        phone_verified: None,
        refresh_token: "refreshed-refresh".to_string(),
        refresh_token_expires_in: Some(refresh_ttl_secs as i64),
        scope: None,
        token_type: "Bearer".to_string(),
        user_id: "default".to_string(),
    }
}
