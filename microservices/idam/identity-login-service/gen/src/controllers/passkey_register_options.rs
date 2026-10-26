// User-owned controller for handler 'passkey_register_options'.

use crate::handlers::passkey_register_options::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(PasskeyRegisterOptionsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        challenge_id: "example".to_string(),
        options: Default::default(),
    })
}
