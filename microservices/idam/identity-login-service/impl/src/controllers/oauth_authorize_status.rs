//! `GET /oauth/authorize/status`: whether a pending authorization request (the hosted login page's `request_id`) can
//! still be completed, and where the client starts a fresh sign-in when it cannot.
//!
//! The hosted page asks on load and whenever its tab comes back into view. When the request has expired it sends the
//! browser to the client's registered `initiate_login_uri` before the user types anything, so nobody signs in only to
//! be told the request is gone. Read-only: the request is not used or extended, and stays single-use.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::oauth_authorize_status::Request;

use crate::services::client_registry::ClientRegistry;
use crate::services::oidc_authorization::authorization_session_remaining;

#[handler(OauthAuthorizeStatusController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let request_id = req.data.request_id.trim();
    let remaining = if request_id.is_empty() {
        Ok(None)
    } else {
        authorization_session_remaining(request_id)
    };
    match remaining {
        Ok(remaining) => HttpJson::new(
            200,
            status_body(
                remaining,
                registered_initiate_login_uri(&req.data.client_id),
            ),
        ),
        Err(error) => {
            tracing::error!(%error, "authorization session store unavailable");
            HttpJson::new(
                503,
                serde_json::json!({
                    "error": "temporarily_unavailable",
                    "error_description": "Authorization service is temporarily unavailable",
                }),
            )
        }
    }
}

/// The active client's registered sign-in start, or `None` (unknown or inactive client, none registered, or the
/// registry could not be read: the page then keeps its message rather than guessing a destination).
fn registered_initiate_login_uri(client_id: &str) -> Option<String> {
    match ClientRegistry::resolve_active(client_id, sesame_idam_database::db()) {
        Ok(binding) => binding.initiate_login_uri,
        Err(error) => {
            tracing::debug!(?error, "no initiate_login_uri for status");
            None
        }
    }
}

/// The response body: `active`, seconds left (`0` when inactive) and the registered `initiate_login_uri` (or null).
#[must_use]
pub fn status_body(
    remaining: Option<u64>,
    initiate_login_uri: Option<String>,
) -> serde_json::Value {
    serde_json::json!({
        "active": remaining.is_some(),
        "expires_in": remaining.unwrap_or(0),
        "initiate_login_uri": initiate_login_uri,
    })
}

#[cfg(test)]
mod tests {
    use super::status_body;

    #[test]
    fn active_request_reports_seconds_left_and_restart_uri() {
        let body = status_body(
            Some(1234),
            Some("https://trader.example/auth/start".to_string()),
        );
        assert_eq!(body["active"], true);
        assert_eq!(body["expires_in"], 1234);
        assert_eq!(
            body["initiate_login_uri"],
            "https://trader.example/auth/start"
        );
    }

    #[test]
    fn expired_request_without_registered_restart_is_null() {
        let body = status_body(None, None);
        assert_eq!(body["active"], false);
        assert_eq!(body["expires_in"], 0);
        assert!(body["initiate_login_uri"].is_null());
    }
}
