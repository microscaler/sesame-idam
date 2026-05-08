// User-owned controller for handler 'validate_api_key'.

use crate::handlers::validate_api_key::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(ValidateApiKeyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        api_key_id: Some("example".to_string()),
        expires_at: Some(42),
        is_expired: Some(true),
        org_id: Some("example".to_string()),
        permissions: Some(vec![]),
        scope_type: Some("example".to_string()),
        user_id: Some("example".to_string()),
        valid: true,
    }
}
