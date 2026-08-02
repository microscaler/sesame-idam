// User-owned controller for handler 'oauth_userinfo'.

use crate::handlers::oauth_userinfo::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(OauthUserinfoController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        email: Some(Default::default()),
        email_verified: Some(Default::default()),
        family_name: Some(Default::default()),
        given_name: Some(Default::default()),
        name: Some(Default::default()),
        preferred_username: Some(Default::default()),
        sub: "example".to_string(),
    })
}
