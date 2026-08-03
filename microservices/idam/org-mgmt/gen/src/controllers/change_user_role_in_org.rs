// User-owned controller for handler 'change_user_role_in_org'.

use crate::handlers::change_user_role_in_org::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(ChangeUserRoleInOrgController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        org_id: "example".to_string(),
        primary_role: "example".to_string(),
        user_id: "example".to_string(),
    })
}
