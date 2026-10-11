// BRRTRouter: user-owned
//! `POST /verify`: verify one SAML response for one connection.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_saml_verifier::{verify, VerifyRequest};
use sesame_saml_verifier_gen::handlers::verify_response::Request;

#[handler(VerifyResponseController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let r = req.data;
    let request = VerifyRequest {
        saml_response: r.saml_response,
        idp_entity_id: r.idp_entity_id,
        idp_certificates: r.idp_certificates,
        sp_entity_id: r.sp_entity_id,
        acs_url: r.acs_url,
        expected_request_id: r.expected_request_id,
        max_clock_skew_secs: r.max_clock_skew_secs.map(i64::from),
    };
    match verify(&request) {
        Ok(assertion) => {
            // Identifiers only: never NameIDs, attributes or input.
            tracing::info!(assertion_id = %assertion.assertion_id, "saml response verified");
            HttpJson::ok(serde_json::to_value(&assertion).unwrap_or_default())
        }
        Err(refusal) => {
            tracing::info!(reason = %refusal, "saml response refused");
            HttpJson::new(422, serde_json::json!({ "reason": refusal.code() }))
        }
    }
}
