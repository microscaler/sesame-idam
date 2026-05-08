// User-owned controller for handler 'clear_user_password'.

use crate::handlers::clear_user_password::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(ClearUserPasswordController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
