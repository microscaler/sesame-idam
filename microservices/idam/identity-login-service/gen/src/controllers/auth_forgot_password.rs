// User-owned controller for handler 'auth_forgot_password'.

use crate::handlers::auth_forgot_password::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(AuthForgotPasswordController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        expires_in: Some(42),
        message: "example".to_string(),
        success: true,
        token_type: Some("example".to_string()),
    })
}
