// User-owned controller for handler 'create_organization'.

use crate::handlers::create_organization::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(CreateOrganizationController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        id: Some("example".to_string()),
        name: Some("example".to_string()),
        tenant_id: Some("example".to_string()),
    })
}
