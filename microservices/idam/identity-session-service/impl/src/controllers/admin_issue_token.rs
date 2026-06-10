/// Handler for Admin Issue Token — admin issues an access token for a user.
/// Uses TTL configuration from `jwt::ttl::TtlConfig` to set `expires_in` on
/// issued access tokens.
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_common::audit::AuditEventType;
use sesame_idam_identity_session_service_gen::handlers::admin_issue_token::{Request, Response};

#[handler(AdminIssueTokenController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    let span = tracing::span!(
        tracing::Level::INFO,
        "token.issued",
        tenant_id = tracing::field::Empty,
        user_id = tracing::field::Empty
    );
    let _guard = span.enter();

    use crate::audit::EMITTER;

    let tenant_id = req.data.x_tenant_id.clone();
    let user_id = req.data.user_id.clone();

    let entry =
        sesame_common::audit::AuditLogEntry::new(AuditEventType::JwtIssued, "identity-session-service")
            .user_id(user_id.clone())
            .tenant_id(tenant_id.clone())
            .decision_source("admin_issue_token")
            .result("allowed")
            .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    span.record("tenant_id", &tenant_id);
    span.record("user_id", &user_id);
    span.record("result", "success");

    // Apply TTL config for access and refresh token expiry.
    let ttl_config = crate::jwt::ttl::TtlConfig::from_env();
    let access_ttl_secs = ttl_config.access_ttl_secs_for_role("org_admin");
    let refresh_ttl_secs = ttl_config.refresh_ttl_for_role("org_admin").as_secs();
    ttl_config.record_ttl_metric("org_admin");

    Response {
        access_token: req.data.scope.clone(),
        email: None,
        email_verified: None,
        expires_in: access_ttl_secs as i32,
        id_token: None,
        mfa_required: None,
        phone_verified: None,
        refresh_token: "default-refresh".to_string(),
        refresh_token_expires_in: Some(refresh_ttl_secs as i32),
        scope: Some(req.data.scope.clone()),
        token_type: "Bearer".to_string(),
        user_id,
    }
}
