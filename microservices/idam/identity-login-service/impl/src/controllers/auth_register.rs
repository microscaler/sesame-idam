// Implementation stub for handler 'auth_register'
use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use sesame_idam_identity_login_service_gen::handlers::auth_register::{Request, Response};

/// Handler for Auth Register.
///
/// Uses role-based TTL configuration from `jwt::ttl::TtlConfig` to set
/// `expires_in` on issued access tokens. All roles use 5-minute (300s)
/// TTL after F-010 alignment.
#[handler(AuthRegisterController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use crate::jwt::ttl::TtlConfig;
    use sesame_common::audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    // Load TTL configuration from env vars (with defaults).
    let ttl_config = TtlConfig::from_env();

    let user_id = Uuid::new_v7();

    // TODO: Validate password strength (min length, complexity)
    // TODO: Hash password with bcrypt/argon2
    // TODO: INSERT INTO users (tenant_id, email, username, password_hash)
    // TODO: Check if email is already in use (return 409 Conflict)
    // TODO: Send email confirmation
    // TODO: Issue access_token + refresh_token
    // TODO: Emit user_created audit event

    // Register assigns a "customer" role by default.
    let access_ttl_secs = ttl_config.access_ttl_secs_for_role("customer");
    let refresh_ttl_secs = ttl_config.refresh_ttl_for_role("customer").as_secs();

    Response {
        access_token: format!("access_{}", Uuid::new_v4()),
        token_type: "Bearer".to_string(),
        expires_in: access_ttl_secs as i32,
        refresh_token: format!("refresh_{}", Uuid::new_v4()),
        refresh_token_expires_in: Some(refresh_ttl_secs as i64),
        user_id: user_id.to_string(),
        email: Some(req.inner.email),
        email_verified: Some(false),
        phone_verified: None,
        mfa_required: Some(false),
        id_token: None,
        scope: "".to_string(),
    }
}
