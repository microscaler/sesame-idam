// User-owned controller for handler 'disable_user'.

use crate::handlers::disable_user::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(DisableUserController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
