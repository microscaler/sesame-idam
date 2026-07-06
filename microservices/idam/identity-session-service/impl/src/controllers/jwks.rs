/// Implementation for handler 'jwks'
/// Serves Ed25519 public keys in JWKS format (RFC 7517).
/// Cache-Control and security headers are injected by `JwksHeadersMiddleware`
/// in `impl/src/middleware/jwks_headers.rs`.
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::jwks::{Request, Response};

use crate::key_manager::KEY_MANAGER;

/// JWKS endpoint handler — serves all current, next, and grace public keys.
///
/// Returns a `Response` with a `keys` array of `JwkOnly` structs.
/// Security headers (`Cache-Control`, `X-Content-Type-Options`, `Vary`)
/// are set by `JwksHeadersMiddleware`, not this handler.
///
/// # Panics
///
/// Panics if the `KEY_MANAGER` lock is poisoned.
#[handler(JwksController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    let span = tracing::span!(tracing::Level::INFO, "jwks.document");
    let _guard = span.enter();

    let doc = KEY_MANAGER.read().unwrap().jwks_document();
    let keys_count = doc.keys.len();

    let keys: Vec<serde_json::Value> = doc
        .keys
        .into_iter()
        .map(|jwk| serde_json::to_value(&jwk).unwrap())
        .collect();

    let resp = Response { keys };

    span.record("keys_count", keys_count);
    tracing::info!(keys_count, "JWKS document served");

    resp
}
