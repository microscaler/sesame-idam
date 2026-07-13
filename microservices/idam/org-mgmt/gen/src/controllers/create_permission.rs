// User-owned controller for handler 'create_permission'.

use crate::handlers::create_permission::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(CreatePermissionController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        application_id: "example".to_string(),
        created_at: "example".to_string(),
        description: Some("example".to_string()),
        id: "example".to_string(),
        name: "example".to_string(),
        updated_at: Some("example".to_string()),
    })
}
