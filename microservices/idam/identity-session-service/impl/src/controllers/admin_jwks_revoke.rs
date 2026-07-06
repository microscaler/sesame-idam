/// Admin handler: POST /admin/jwks/revoke
/// Immediately revoke a key by `kid`. Removes from JWKS and drops private key.
/// Implements HACK-101 fix: compromised keys can be revoked at any time.
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::admin_jwks_revoke::{Request, Response};

use crate::key_manager::KeyError;

use crate::key_manager::KEY_MANAGER;

/// Revoke a JWKS key by its `kid`.
///
/// Looks up the key in the key manager's current + next + previous slots.
/// If found, removes it from JWKS and drops the private key from memory.
///
/// Returns success on revoke, `key_not_found` if the kid doesn't match
/// any live or grace key, or `revocation_failed` for lock/other errors.
#[handler(AdminRevokeKeyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    let kid = req.data.kid.clone();
    let span = tracing::span!(tracing::Level::INFO, "key.revoke.admin", kid = &kid);
    let _guard = span.enter();

    let result = match KEY_MANAGER.write() {
        Ok(mut guard) => guard.revoke_key(&kid).map_err(|e| {
            tracing::error!(error = ?e, "KEY_MANAGER.revoke_key failed");
            e
        }),
        Err(poison) => {
            tracing::error!(error = ?poison, "KEY_MANAGER write lock poisoned");
            Err(KeyError::RevocationFailed("lock poisoned".to_string()))
        }
    };

    match result {
        Ok(()) => {
            tracing::info!(kid = &kid, "admin: key revoked successfully");
            span.record("result", "success");
            Response {
                kid: Some(kid),
                message: Some("Key revoked successfully".to_string()),
                success: Some(true),
            }
        }
        Err(KeyError::KeyNotFound(_)) => {
            tracing::warn!(kid = &kid, "admin: revocation requested for unknown key");
            span.record("result", "key_not_found");
            Response {
                kid: Some(kid),
                message: Some("Key not found".to_string()),
                success: Some(false),
            }
        }
        Err(KeyError::RevocationFailed(msg)) => {
            tracing::warn!(kid = &kid, error = &msg, "admin: revocation failed");
            span.record("result", "revocation_failed");
            Response {
                kid: Some(kid),
                message: Some(format!("Key revocation failed: {msg}")),
                success: Some(false),
            }
        }
        Err(e) => {
            tracing::error!(kid = &kid, error = ?e, "admin: revocation failed unexpectedly");
            span.record("result", "error");
            Response {
                kid: Some(kid),
                message: Some(format!("Revocation failed: {e}")),
                success: Some(false),
            }
        }
    }
}
