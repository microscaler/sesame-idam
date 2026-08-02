// User-owned controller for handler 'oauth_userinfo'.

use crate::handlers::oauth_userinfo::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(OauthUserinfoController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        avatar_url: Some(Default::default()),
        email: Some(Default::default()),
        email_verified: Some(true),
        first_name: Some(Default::default()),
        last_name: Some(Default::default()),
        name: Some(Default::default()),
        org_id: Some(Default::default()),
        org_name: Some(Default::default()),
        phone_number: Some(Default::default()),
        phone_verified: Some(true),
        preferred_username: Some(Default::default()),
        properties: Some(Default::default()),
        sub: Some("example".to_string()),
        updated_at: Some(Default::default()),
        user_id: Some(Default::default()),
        user_permissions: Some(Default::default()),
        user_role: Some(Default::default()),
        username: Some(Default::default()),
    })
}
