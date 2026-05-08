// User-owned controller for handler 'fetch_api_key_usage'.

use crate::handlers::fetch_api_key_usage::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(FetchApiKeyUsageController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        date: Some("example".to_string()),
        total_validations: Some(42),
    }
}
