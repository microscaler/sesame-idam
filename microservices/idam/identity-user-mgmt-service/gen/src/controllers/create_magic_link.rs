// User-owned controller for handler 'create_magic_link'.

use crate::handlers::create_magic_link::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(CreateMagicLinkController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
