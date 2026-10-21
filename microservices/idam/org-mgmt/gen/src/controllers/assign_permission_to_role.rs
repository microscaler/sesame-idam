// User-owned controller for handler 'assign_permission_to_role'.

use crate::handlers::assign_permission_to_role::Request;
use brrtrouter::typed::HttpNoContent;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(AssignPermissionToRoleController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpNoContent {
    HttpNoContent
}
