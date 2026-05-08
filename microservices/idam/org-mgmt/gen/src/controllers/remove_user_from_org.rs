// User-owned controller for handler 'remove_user_from_org'.

use crate::handlers::remove_user_from_org::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(RemoveUserFromOrgController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
