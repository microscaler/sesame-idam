// User-owned controller for handler 'disable_user_mfa'.

use crate::handlers::disable_user_mfa::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(DisableUserMfaController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        factors_removed: 42,
        user_id: "example".to_string(),
    })
}
