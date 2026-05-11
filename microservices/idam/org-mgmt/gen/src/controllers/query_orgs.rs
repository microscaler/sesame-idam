// User-owned controller for handler 'query_orgs'.

use crate::handlers::query_orgs::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(QueryOrgsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
