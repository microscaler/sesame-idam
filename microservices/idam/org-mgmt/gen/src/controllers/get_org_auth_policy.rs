// User-owned controller for handler 'get_org_auth_policy'.

use crate::handlers::get_org_auth_policy::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(GetOrgAuthPolicyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        org_id: "example".to_string(),
        require: "example".to_string(),
        updated_at: Some(42),
    })
}
