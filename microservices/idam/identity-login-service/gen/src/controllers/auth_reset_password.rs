// User-owned controller for handler 'auth_reset_password'.

use crate::handlers::auth_reset_password::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(AuthResetPasswordController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        message: Some("example".to_string()),
        success: Some(true),
    }
}
