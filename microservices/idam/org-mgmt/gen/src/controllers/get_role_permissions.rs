// User-owned controller for handler 'get_role_permissions'.

use crate::handlers::get_role_permissions::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::Permission;

#[handler(GetRolePermissionsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        items: vec![],
        page: 42,
        page_size: 42,
        total: 42,
    }
}
