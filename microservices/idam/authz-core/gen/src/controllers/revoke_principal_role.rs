// User-owned controller for handler 'revoke_principal_role'.

use crate::handlers::revoke_principal_role::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(RevokePrincipalRoleController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
