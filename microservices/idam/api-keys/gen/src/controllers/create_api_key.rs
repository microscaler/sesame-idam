// User-owned controller for handler 'create_api_key'.

use crate::handlers::create_api_key::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(CreateApiKeyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        api_key: "example".to_string(),
        api_key_id: "example".to_string(),
        created_at: Some(42),
        expires_at: Some(42),
        name: Some("example".to_string()),
        org_id: Some("example".to_string()),
        permissions: Some(vec![]),
        user_id: Some("example".to_string()),
    }
}
