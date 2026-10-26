// User-owned controller for handler 'mfa_manage'.

use crate::handlers::mfa_manage::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(MfaManageController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        expires_in: 42,
        mfa_token: "example".to_string(),
    })
}
