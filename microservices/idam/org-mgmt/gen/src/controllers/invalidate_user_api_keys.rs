// User-owned controller for handler 'invalidate_user_api_keys'.

use crate::handlers::invalidate_user_api_keys::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(InvalidateUserApiKeysController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        invalidated: 42,
        message: Some("example".to_string()),
    })
}
