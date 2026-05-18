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
    match KEY_MANAGER.revoke_key(&req.kid) {
        Ok(()) => RevokeKeyResponse {
            success: true,
            kid: req.kid,
            message: format!("Key {} revoked and removed from JWKS immediately", req.kid),
        },
        Err(e) => RevokeKeyResponse {
            success: false,
            kid: req.kid,
            message: format!("Revocation failed: {e}"),
        },
    }
}
