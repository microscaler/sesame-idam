// BRRTRouter: user-owned

use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::auth_logout::{Request, Response};

/// Handler for Auth Logout — revokes the refresh token family in Redis.
#[handler(AuthLogoutController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEventType, AuditLogEntry};

    let tenant_id = req.data.x_tenant_id.clone();

    if let Some(refresh_token) = req.data.refresh_token.as_deref() {
        if let Err(e) = crate::redis::revoke_refresh_token(refresh_token) {
            tracing::warn!(
                error = %e,
                tenant_id = %tenant_id,
                "logout: failed to revoke refresh token in Redis"
            );
        }
    }

    let entry = AuditLogEntry::new(AuditEventType::TokenRevoked, "identity-login-service")
        .tenant_id(tenant_id)
        .decision_source("auth_logout")
        .result("allowed")
        .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    Response {
        error: String::new(),
        error_description: None,
        hint: None,
        retry_after: None,
    }
}
