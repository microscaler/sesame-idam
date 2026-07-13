// User-owned controller for handler 'fetch_scim_groups'.

use crate::handlers::fetch_scim_groups::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::ScimGroup;

#[handler(FetchScimGroupsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        items: vec![],
        page: 42,
        page_size: 42,
        total: 42,
    })
}
