// User-owned controller for handler 'scim_delete_user'.

use crate::handlers::scim_delete_user::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(ScimDeleteUserController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        detail: "example".to_string(),
        schemas: vec![],
        scim_type: Some("example".to_string()),
        status: "example".to_string(),
    })
}
