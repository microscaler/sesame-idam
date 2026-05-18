// Admin handler: POST /admin/jwks/revoke
// Immediately revoke a key by `kid`. Removes from JWKS and drops private key.
// Implements HACK-101 fix: compromised keys can be revoked at any time.

use brrtrouter_macros::handler;
use serde::{Deserialize, Serialize};

use crate::key_manager::KEY_MANAGER;

#[derive(Debug, Deserialize)]
pub struct RevokeKeyRequest {
    pub kid: String,
}

#[derive(Debug, Serialize)]
pub struct RevokeKeyResponse {
    pub success: bool,
    pub kid: String,
    pub message: String,
}

#[handler(AdminRevokeKeyController)]
pub fn handle(req: RevokeKeyRequest) -> RevokeKeyResponse {
    // Span: track revocation events
    let span = tracing::span!(
        tracing::Level::INFO,
        "key.revoke.admin",
        kid = &req.kid
    );
    let _guard = span.enter();

    match KEY_MANAGER.revoke_key(&req.kid) {
        Ok(()) => {
            tracing::info!(kid = req.kid, "admin: key revoked via admin endpoint");
            span.record("result", "success");
            RevokeKeyResponse {
                success: true,
                kid: req.kid,
                message: "Key revoked and removed from JWKS immediately".to_string(),
            }
        }
        Err(e) => {
            tracing::warn!(kid = req.kid, error = %e, "admin: key revocation failed");
            span.record("result", "denied");
            span.record("error", e.to_string().as_str());
            RevokeKeyResponse {
                success: false,
                kid: req.kid,
                message: format!("Revocation failed: {e}"),
            }
        }
    }
}
