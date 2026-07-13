// User-owned controller for handler 'fetch_scim_group'.

use crate::handlers::fetch_scim_group::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(FetchScimGroupController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        created_at: Some("example".to_string()),
        description: Some("example".to_string()),
        id: "example".to_string(),
        members: vec![],
        name: "example".to_string(),
        updated_at: Some("example".to_string()),
    })
}
