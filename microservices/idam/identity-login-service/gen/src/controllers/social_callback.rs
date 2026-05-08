// User-owned controller for handler 'social_callback'.

use crate::handlers::social_callback::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(SocialCallbackController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        access_token: "example".to_string(),
        email: Some("example".to_string()),
        email_verified: Some(true),
        expires_in: 42,
        refresh_token: "example".to_string(),
        social_provider: "example".to_string(),
        social_provider_user_id: Some("example".to_string()),
        token_type: "example".to_string(),
        user_id: "example".to_string(),
    }
}
