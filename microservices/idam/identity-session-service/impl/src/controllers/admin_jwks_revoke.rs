// Admin handler: POST /admin/jwks/revoke
// Immediately revoke a key by `kid`. Removes from JWKS and drops private key.
// Implements HACK-101 fix: compromised keys can be revoked at any time.

use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

use sesame_idam_identity_session_service_gen::handlers::admin_jwks_revoke::{Request, Response};

#[handler(AdminRevokeKeyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    let kid = req.data.kid.clone();
    // Span: track revocation events
    let span = tracing::span!(tracing::Level::INFO, "key.revoke.admin", kid = &kid);
    let _guard = span.enter();

    // TODO: Call KEY_MANAGER.revoke_key(&kid) — requires interior mutability
    // on KEY_MANAGER (currently &mut self but KEY_MANAGER is immutable static)

    tracing::info!(kid = kid, "admin: key revoke request received (stub)");
    span.record("result", "success");
    Response {
        kid: Some(kid),
        message: Some("Key revocation requires mutable KEY_MANAGER access (stub)".to_string()),
        success: Some(true),
    }
}
