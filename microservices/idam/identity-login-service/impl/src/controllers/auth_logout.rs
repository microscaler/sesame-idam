// BRRTRouter: user-owned

use brrtrouter::dispatcher::{HandlerRequest, HandlerResponse, HeaderVec};
use brrtrouter::typed::{TypedHandlerFor, TypedHandlerRequest};
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

    // Denylist the presented access token so it cannot be used until it expires.
    // The bearer is JWKS-validated upstream, so `jwt_claims` carries a trusted
    // `jti`/`exp`; TTL is the token's remaining lifetime (bounded by access TTL).
    if let Some(claims) = req.jwt_claims.as_ref() {
        if let Some(jti) = claims.get("jti").and_then(|v| v.as_str()) {
            let ttl = access_token_remaining_ttl(claims);
            if let Err(e) = crate::redis::deny_access_jti(jti, ttl) {
                tracing::warn!(
                    error = %e,
                    tenant_id = %tenant_id,
                    "logout: failed to denylist access jti in Redis"
                );
            }
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

/// HTTP boundary for logout.
///
/// The generated typed controller maps a serialized response to HTTP 200, but
/// the `OpenAPI` contract deliberately defines successful logout as `204 No
/// Content`. Register this untyped adapter at the dispatcher boundary so the
/// wire response matches the contract while the typed handler remains reusable
/// by the BDD suite.
#[must_use]
pub fn handle_http(req: HandlerRequest) -> HandlerResponse {
    match TypedHandlerRequest::<Request>::from_handler(req) {
        Ok(typed_req) => {
            let _ = handle(typed_req);
            HandlerResponse::new(204, HeaderVec::new(), serde_json::Value::Null)
        }
        Err(error) => HandlerResponse::json(
            400,
            serde_json::json!({
                "error": "invalid_request",
                "error_description": error.to_string(),
            }),
        ),
    }
}

/// Remaining lifetime (seconds) of the access token from its `exp` claim, so the
/// denylist entry lives exactly as long as the token could still be accepted.
/// Capped against clock-skewed/forged `exp`; falls back to the normal access TTL
/// when `exp` is absent or already elapsed.
fn access_token_remaining_ttl(claims: &serde_json::Value) -> u64 {
    const FALLBACK_TTL_SECS: u64 = 300; // normal access-token TTL
    const MAX_DENY_TTL_SECS: u64 = 3600; // safety cap against skewed/forged exp

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());

    claims
        .get("exp")
        .and_then(serde_json::Value::as_u64)
        .map(|exp| exp.saturating_sub(now))
        .filter(|&ttl| ttl > 0)
        .map_or(FALLBACK_TTL_SECS, |ttl| ttl.min(MAX_DENY_TTL_SECS))
}
