// User-owned controller for handler 'oauth_token'.

use crate::handlers::oauth_token::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(OauthTokenController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        access_token: "example".to_string(),
        expires_in: 42,
        id_token: Some("example".to_string()),
        refresh_token: Some("example".to_string()),
        refresh_token_expires_in: Some(42),
        scope: "example".to_string(),
        token_type: "example".to_string(),
    })
}
