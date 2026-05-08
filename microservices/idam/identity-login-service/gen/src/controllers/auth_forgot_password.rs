// User-owned controller for handler 'auth_forgot_password'.

use crate::handlers::auth_forgot_password::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(AuthForgotPasswordController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        message: Some("example".to_string()),
        success: Some(true),
    }
}
