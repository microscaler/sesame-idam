// User-owned controller for handler 'oauth_authorize_status'.

use crate::handlers::oauth_authorize_status::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(OauthAuthorizeStatusController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        active: true,
        expires_in: 42,
        initiate_login_uri: Some("example".to_string()),
    })
}
