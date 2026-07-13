// User-owned controller for handler 'fetch_role_mappings'.

use crate::handlers::fetch_role_mappings::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(FetchRoleMappingsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        assigned_roles: Some(vec![]),
        mapping_name: Some("example".to_string()),
        org_id: Some("example".to_string()),
        subscribed_at: Some("example".to_string()),
    })
}
