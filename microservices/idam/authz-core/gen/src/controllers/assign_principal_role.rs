// User-owned controller for handler 'assign_principal_role'.

use crate::handlers::assign_principal_role::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(AssignPrincipalRoleController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
