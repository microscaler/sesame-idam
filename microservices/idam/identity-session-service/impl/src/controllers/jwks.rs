// Implementation for handler 'jwks'
// Serves Ed25519 public keys in JWKS format (RFC 7517).
// Includes Cache-Control and security headers via BRRTRouter response interceptor.

use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::jwks::{Request, Response};

use crate::key_manager::KEY_MANAGER;

/// Cache-Control header value for JWKS responses.
/// Consumers should cache this for 5 minutes to avoid excessive fetches.
const JWKS_CACHE_CONTROL: &str = "public, max-age=300";

/// X-Content-Type-Options header to prevent MIME sniffing.
const X_CONTENT_TYPE_OPTIONS: &str = "nosniff";

/// Vary header to ensure CDN/proxy caching works correctly.
const VARY: &str = "Accept";

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

/// Build JWKS response with headers (standalone, for testing).
pub fn serve_with_headers() -> (Response, std::collections::HashMap<String, String>) {
    let doc = KEY_MANAGER.read().unwrap().jwks_document();
    let keys: Vec<serde_json::Value> = doc
        .keys
        .into_iter()
        .map(|jwk| serde_json::to_value(&jwk).unwrap())
        .collect();

    let resp = Response { keys };

    let mut headers = std::collections::HashMap::new();
    headers.insert("Cache-Control".to_string(), JWKS_CACHE_CONTROL.to_string());
    headers.insert(
        "X-Content-Type-Options".to_string(),
        X_CONTENT_TYPE_OPTIONS.to_string(),
    );
    headers.insert("Vary".to_string(), VARY.to_string());

    (resp, headers)
}
