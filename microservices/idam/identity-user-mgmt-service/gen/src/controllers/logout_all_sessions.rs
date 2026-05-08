// User-owned controller for handler 'logout_all_sessions'.

use crate::handlers::logout_all_sessions::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(LogoutAllSessionsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
