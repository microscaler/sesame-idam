// User-owned controller for handler 'update_org'.

use crate::handlers::update_org::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(UpdateOrgController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
