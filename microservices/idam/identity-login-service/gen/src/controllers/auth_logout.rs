// User-owned controller for handler 'auth_logout'.

use crate::handlers::auth_logout::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(AuthLogoutController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        error: "example".to_string(),
        error_description: Some("example".to_string()),
        hint: Some("example".to_string()),
        retry_after: Some(42),
    }
}
