// User-owned controller for handler 'get_application'.

use crate::handlers::get_application::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(GetApplicationController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        created_at: "example".to_string(),
        id: "example".to_string(),
        name: "example".to_string(),
        org_id: Some("example".to_string()),
        slug: "example".to_string(),
        updated_at: Some("example".to_string()),
    }
}
