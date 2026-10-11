// BRRTRouter: user-owned
//! `POST /metadata/parse`: parse `IdP` metadata into what a connection stores.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_saml_verifier::metadata::parse_metadata;
use sesame_saml_verifier_gen::handlers::parse_metadata::Request;

#[handler(ParseMetadataController)]
#[allow(clippy::needless_pass_by_value)] // the handler signature is fixed by the macro
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    match parse_metadata(&req.data.metadata) {
        Ok(metadata) => HttpJson::ok(serde_json::to_value(&metadata).unwrap_or_default()),
        Err(refusal) => {
            tracing::info!(reason = %refusal, "idp metadata refused");
            HttpJson::new(422, serde_json::json!({ "reason": refusal.code() }))
        }
    }
}
