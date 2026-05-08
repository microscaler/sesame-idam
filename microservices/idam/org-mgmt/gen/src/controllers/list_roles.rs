// User-owned controller for handler 'list_roles'.

use crate::handlers::list_roles::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::Role;

#[handler(ListRolesController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        items: vec![],
        page: 42,
        page_size: 42,
        total: 42,
    }
}
