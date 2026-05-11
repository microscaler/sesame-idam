// User-owned controller for handler 'list_permissions'.

use crate::handlers::list_permissions::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(ListPermissionsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
