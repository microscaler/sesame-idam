// User-owned controller for handler 'oauth_userinfo'.

use crate::handlers::oauth_userinfo::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(OauthUserinfoController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        email: Some("example".to_string()),
        email_verified: Some(true),
        first_name: Some("example".to_string()),
        last_name: Some("example".to_string()),
        name: Some("example".to_string()),
        org_id: Some("example".to_string()),
        org_name: Some("example".to_string()),
        phone_number: Some("example".to_string()),
        phone_verified: Some(true),
        picture_url: Some("example".to_string()),
        preferred_username: Some("example".to_string()),
        properties: Some(Default::default()),
        sub: Some("example".to_string()),
        updated_at: Some("example".to_string()),
        user_id: Some("example".to_string()),
        user_permissions: Some(vec![]),
        user_role: Some("example".to_string()),
        username: Some("example".to_string()),
    }
}
