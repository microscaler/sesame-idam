// User-owned controller for handler 'list_principal_roles'.

use crate::handlers::list_principal_roles::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::PrincipalRoleAssignment;

#[handler(ListPrincipalRolesController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        assignments: vec![],
    })
}
