// User-owned controller for handler 'delete_org'.

use crate::handlers::delete_org::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(DeleteOrgController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
