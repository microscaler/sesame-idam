// User-owned controller for handler 'fetch_archived_api_key'.

use crate::handlers::fetch_archived_api_key::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(FetchArchivedApiKeyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        archived_reason: Some("example".to_string()),
        reason: Some("example".to_string()),
        revoked_at: Some(42),
        revoked_by_user_id: Some("example".to_string()),
    }
}
