// User-owned controller for handler 'list_my_memberships'.

use crate::handlers::list_my_memberships::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(ListMyMembershipsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response(vec![])
}
