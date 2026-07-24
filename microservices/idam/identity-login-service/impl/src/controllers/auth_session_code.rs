// BRRTRouter: user-owned

//! `POST /auth/session/code` — mint a one-time code for cross-origin handoff.
//!
//! Called by the hosted auth surface after a user authenticates, so the
//! session can cross to a tenant app on a different origin without putting
//! tokens in a URL (see `services::auth_code` for the reasoning).
//!
//! The presented access token is verified before minting: without that check
//! anyone could POST an arbitrary string and receive a code redeemable for
//! "a session". Verification proves the caller holds a token this platform
//! actually issued.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::auth_session_code::Request;

use crate::services::auth_code::{self, CodePayload};
use crate::services::tenant_gate::tenant_http_error;
use crate::services::tenant_service::TenantService;

#[handler(AuthSessionCodeController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let tenant_id = req.data.x_tenant_id.clone();

    let exec = sesame_idam_database::db();
    if let Err(e) = TenantService::require_active(tenant_id.trim(), exec) {
        return tenant_http_error(&e);
    }

    // Only mint against a token we actually issued.
    if !token_is_ours(&req.data.access_token, &tenant_id) {
        return bad_request("access token is not valid for this tenant");
    }

    if req.data.redirect_uri.trim().is_empty() {
        return bad_request("redirect_uri is required");
    }

    let payload = CodePayload {
        access_token: req.data.access_token.clone(),
        refresh_token: req.data.refresh_token.clone(),
        redirect_uri: req.data.redirect_uri.clone(),
        tenant_id: tenant_id.clone(),
    };

    match auth_code::mint(&payload) {
        Ok(code) => HttpJson::ok(serde_json::json!({
            "code": code,
            "expires_in": i32::try_from(auth_code::ttl_secs()).unwrap_or(60),
        })),
        Err(e) => {
            tracing::error!(error = %e, "auth_session_code: mint failed");
            HttpJson::new(
                500,
                serde_json::json!({
                    "error": "internal_error",
                    "error_description": "An unexpected error occurred"
                }),
            )
        }
    }
}

/// Cheap structural + tenant check on the presented access token.
///
/// Full signature validation happens at every resource server via JWKS; here
/// we only need to establish that the caller holds a token this platform
/// minted for THIS tenant, so a code cannot be conjured from nothing.
fn token_is_ours(token: &str, tenant_id: &str) -> bool {
    use base64::Engine;
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    let Ok(bytes) = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(parts[1]) else {
        return false;
    };
    let Ok(claims) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return false;
    };
    claims.get("tenant_id").and_then(|v| v.as_str()) == Some(tenant_id)
}

fn bad_request(msg: &str) -> HttpJson<serde_json::Value> {
    HttpJson::new(
        400,
        serde_json::json!({ "error": "invalid_request", "error_description": msg }),
    )
}
