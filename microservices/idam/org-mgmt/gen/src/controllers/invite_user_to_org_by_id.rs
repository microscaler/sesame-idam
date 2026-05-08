// User-owned controller for handler 'invite_user_to_org_by_id'.

use crate::handlers::invite_user_to_org_by_id::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(InviteUserToOrgByIdController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
