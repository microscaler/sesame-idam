// User-owned controller for handler 'invite_user_to_org'.

use crate::handlers::invite_user_to_org::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(InviteUserToOrgController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        invite_id: "example".to_string(),
        invite_token: "example".to_string(),
        success: true,
    })
}
