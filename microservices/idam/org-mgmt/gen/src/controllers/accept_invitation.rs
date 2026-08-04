// User-owned controller for handler 'accept_invitation'.

use crate::handlers::accept_invitation::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(AcceptInvitationController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        id: "example".to_string(),
        name: "example".to_string(),
        tenant_id: "example".to_string(),
    })
}
