// User-owned controller for handler 'delete_retention_policy'.

use crate::handlers::delete_retention_policy::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(DeleteRetentionPolicyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        error: "example".to_string(),
        error_description: Some("example".to_string()),
    })
}
