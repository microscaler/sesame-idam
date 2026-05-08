// User-owned controller for handler 'revoke_permission_from_role'.

use crate::handlers::revoke_permission_from_role::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(RevokePermissionFromRoleController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
