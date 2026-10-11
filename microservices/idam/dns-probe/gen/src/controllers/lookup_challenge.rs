// User-owned controller for handler 'lookup_challenge'.

use crate::handlers::lookup_challenge::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::ProviderAnswer;

#[handler(LookupChallengeController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        answers: vec![],
        name: "example".to_string(),
    })
}
