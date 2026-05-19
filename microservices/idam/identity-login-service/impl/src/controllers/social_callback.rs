use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use sesame_idam_identity_login_service_gen::handlers::social_callback::{Request, Response};

/// Handler for Social Callback.
#[handler(SocialCallbackController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    // TODO: Exchange auth code for OAuth2 tokens from social provider (Google, Apple, GitHub, etc.)
    // TODO: Validate ID token signature and issuer
    // TODO: Extract user info (email, name, profile picture)
    // TODO: Check if user exists (by provider user ID)
    //   - If exists: log them in (issue JWT)
    //   - If not: link to existing user (by email) or create new
    // TODO: Store social account link in user_social_accounts table
    // TODO: Issue access_token + refresh_token
    // TODO: Emit social_login audit event

    Response {
        access_token: format!("access_{}", Uuid::new_v4()),
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        refresh_token: format!("refresh_{}", Uuid::new_v4()),
        email: None,
        email_verified: None,
        user_id: Uuid::new_v7().to_string(),
        social_provider: req.inner.provider,
        social_provider_user_id: None,
    }
}
