// User-owned controller for handler 'create_application'.

use crate::handlers::create_application::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(CreateApplicationController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        created_at: "example".to_string(),
        id: "example".to_string(),
        name: "example".to_string(),
        org_id: Some("example".to_string()),
        slug: "example".to_string(),
        updated_at: Some("example".to_string()),
    })
}
