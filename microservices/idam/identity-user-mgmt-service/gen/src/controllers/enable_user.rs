// User-owned controller for handler 'enable_user'.

use crate::handlers::enable_user::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(EnableUserController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
