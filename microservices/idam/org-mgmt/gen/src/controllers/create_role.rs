// User-owned controller for handler 'create_role'.

use crate::handlers::create_role::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(CreateRoleController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        application_id: "example".to_string(),
        created_at: "example".to_string(),
        description: Some("example".to_string()),
        id: "example".to_string(),
        name: "example".to_string(),
        updated_at: Some("example".to_string()),
    }
}
