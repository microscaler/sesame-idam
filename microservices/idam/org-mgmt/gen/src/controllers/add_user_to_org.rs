// User-owned controller for handler 'add_user_to_org'.

use crate::handlers::add_user_to_org::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(AddUserToOrgController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
