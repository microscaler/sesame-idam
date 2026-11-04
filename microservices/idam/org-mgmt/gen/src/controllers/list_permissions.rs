// User-owned controller for handler 'list_permissions'.

use crate::handlers::list_permissions::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::Permission;

#[handler(ListPermissionsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        has_more: true,
        items: vec![],
        page: 42,
        page_size: 42,
        total: 42,
    })
}
