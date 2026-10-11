// User-owned controller for handler 'verify_response'.

use crate::handlers::verify_response::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(VerifyResponseController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        assertion_id: "example".to_string(),
        attributes: Default::default(),
        authn_instant: Some("example".to_string()),
        issuer: "example".to_string(),
        name_id: "example".to_string(),
        name_id_format: Some("example".to_string()),
        not_on_or_after: "example".to_string(),
        session_index: Some("example".to_string()),
        session_not_on_or_after: Some("example".to_string()),
    })
}
