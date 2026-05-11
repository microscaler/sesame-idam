// User-owned controller for handler 'deleteRetentionPolicy'.

use crate::handlers::deleteRetentionPolicy::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(DeleteRetentionPolicyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        error: "example".to_string(),
        error_description: Some("example".to_string()),
    }
}
