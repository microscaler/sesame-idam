// User-owned controller for handler 'jwks'.
//
// NOTE: This gen controller is a placeholder. The actual implementation
// is in `impl/src/controllers/jwks.rs` and is registered via
// `main.rs` after `registry::register_from_spec()`.
//
// To regenerate from OpenAPI spec:
//   cargo run --manifest-path ../BRRTRouter/Cargo.toml --bin brrtrouter-gen \
//     -- generate --spec openapi/identity-session-service/openapi.yaml \
//     --output gen/ --package-name sesame_idam_identity_session_service

use crate::handlers::jwks::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(JwksController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Placeholder - actual implementation delegates to impl controller
    Response {
        keys: vec![
            serde_json::json!({
                "alg": "EdDSA",
                "crv": "Ed25519",
                "kid": "placeholder",
                "kty": "OKP",
                "use": "sig",
                "x": "placeholder"
            }),
        ],
    }
}
