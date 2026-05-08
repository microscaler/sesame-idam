// User-owned controller for handler 'import_api_keys'.

use crate::handlers::import_api_keys::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(ImportApiKeysController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        errors: Some(vec![]),
        failed_count: Some(42),
        imported_count: Some(42),
    }
}
