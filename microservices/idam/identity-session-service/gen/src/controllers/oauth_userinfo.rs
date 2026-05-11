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
        name: Some(Default::default()),
        org_id: Some(Default::default()),
        org_name: Some(Default::default()),
        phone_number: Some(Default::default()),
        phone_verified: Some(true),
        picture_url: Some(Default::default()),
        preferred_username: Some(Default::default()),
        properties: Some(Default::default()),
        sub: Some("example".to_string()),
        updated_at: Some("example".to_string()),
        user_id: Some("example".to_string()),
        user_permissions: Some(Default::default()),
        user_role: Some(Default::default()),
        username: Some("example".to_string()),
    }
}
