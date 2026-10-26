// User-owned controller for handler 'passkey_challenge'.

use crate::handlers::passkey_challenge::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(PasskeyChallengeController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        challenge_id: "example".to_string(),
        options: Default::default(),
    })
}
