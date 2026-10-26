// User-owned controller for handler 'oauth_userinfo'.

use crate::handlers::oauth_userinfo::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(OauthUserinfoController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        email: Some("example".to_string()),
        email_verified: Some(true),
        family_name: Some("example".to_string()),
        given_name: Some("example".to_string()),
        name: Some("example".to_string()),
        preferred_username: Some("example".to_string()),
        sub: "example".to_string(),
    })
}
