// User-owned controller for handler 'revoke_pending_invite'.

use crate::handlers::revoke_pending_invite::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(RevokePendingInviteController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
