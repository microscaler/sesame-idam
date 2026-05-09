// User-owned controller for handler 'update_api_key'.

use crate::handlers::update_api_key::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(UpdateApiKeyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        active: Some(true),
        api_key_id: Some("example".to_string()),
        created_at: Some(42),
        expires_at: Some(42),
        metadata: Some(Default::default()),
        name: Some("example".to_string()),
        org_id: Some("example".to_string()),
        permissions: Some(vec![]),
        user_id: Some("example".to_string()),
    }
}
