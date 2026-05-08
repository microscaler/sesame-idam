// User-owned controller for handler 'assign_permission_to_role'.

use crate::handlers::assign_permission_to_role::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(AssignPermissionToRoleController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
