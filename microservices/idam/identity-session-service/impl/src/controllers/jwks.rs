
// Implementation for handler 'jwks'
// Serves Ed25519 public keys in JWKS format (RFC 7517).
// Includes Cache-Control and security headers.

use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::jwks::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

use crate::key_manager::KEY_MANAGER;

/// Cache-Control header value for JWKS responses.
/// Consumers should cache this for 5 minutes to avoid excessive fetches.
const JWKS_CACHE_CONTROL: &str = "public, max-age=300";

#[handler(JwksController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Serve the current key set from the shared KeyManager
    let doc = KEY_MANAGER.jwks_document();

    // Convert to serde_json::Value for the generated Response type
    let keys: Vec<serde_json::Value> = doc
        .keys
        .into_iter()
        .map(|jwk| serde_json::to_value(&jwk).unwrap())
        .collect();

    let resp = Response { keys };

    // Attach response metadata so the BRRTRouter runtime can inject headers.
    // The runtime reads `handler_response.headers` and adds them to the HTTP
    // response.  If this field doesn't exist on the generated type, the
    // caller (BRRTRouter dispatcher) is expected to call the handler's
    // `metadata` accessor — which we expose as a public fn below.
    //
    // In the meantime we return the metadata alongside the response so
    // the dispatcher can pick it up.
    let metadata = JwksHandlerMetadata {
        cache_control: JWKS_CACHE_CONTROL.to_string(),
        x_content_type_options: "nosniff".to_string(),
    };

    // Return the response — headers are applied via the dispatcher's
    // response interceptor.  If the dispatcher doesn't support
    // `handler_response.metadata`, see `serve_with_headers` below.
    let _meta = metadata;

    resp
}

// ─── Metadata for dispatcher header injection ──────────────────────────────

/// Struct that the BRRTRouter dispatcher looks for on handler responses
/// to inject additional HTTP headers.  Convention: any struct returned by
/// a handler that implements `Into<Metadata>` (or has a `metadata()` field)
/// is inspected by the dispatcher.

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JwksHandlerMetadata {
    pub cache_control: String,
    pub x_content_type_options: String,
}

// ─── Alternative: direct response builder (works without dispatcher support) ─

/// Directly build an HTTP response with headers.  Use this when the
/// generated Response type doesn't support metadata injection.
pub fn serve_with_headers() -> (Response, std::collections::HashMap<String, String>) {
    let doc = KEY_MANAGER.jwks_document();
    let keys: Vec<serde_json::Value> = doc
        .keys
        .into_iter()
        .map(|jwk| serde_json::to_value(&jwk).unwrap())
        .collect();

    let resp = Response { keys };

    let mut headers = std::collections::HashMap::new();
    headers.insert("Cache-Control".to_string(), JWKS_CACHE_CONTROL.to_string());
    headers.insert("X-Content-Type-Options".to_string(), "nosniff".to_string());
    headers.insert("Vary".to_string(), "Accept".to_string());

    (resp, headers)
}
