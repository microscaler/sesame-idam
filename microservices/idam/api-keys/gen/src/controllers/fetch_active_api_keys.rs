// User-owned controller for handler 'fetch_active_api_keys'.

use crate::handlers::fetch_active_api_keys::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::ApiKey;

#[handler(FetchActiveApiKeysController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        current_page: Some(42),
        has_more_results: Some(true),
        keys: Some(vec![]),
        page_size: Some(42),
        total_keys: Some(42),
    }
}
