// User-owned controller for handler 'query_orgs'.

use crate::handlers::query_orgs::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::Org;

#[handler(QueryOrgsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        items: vec![],
        page: 42,
        page_size: 42,
        total: 42,
    }
}
