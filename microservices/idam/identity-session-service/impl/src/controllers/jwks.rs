
// Implementation stub for handler 'jwks'
// This file is a starting point for your implementation.
// You can modify this file freely - it will NOT be auto-regenerated.
// To regenerate this stub, use: brrtrouter-gen generate-stubs --path jwks --force

use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::jwks::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

use crate::key_manager::{JwksDocument, KEY_MANAGER};



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
    
    Response { keys }
}
