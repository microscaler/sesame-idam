// User-owned controller for handler 'oauth_authorize_complete'.

use crate::handlers::oauth_authorize_complete::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(OauthAuthorizeCompleteController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        redirect: "example".to_string(),
    })
}
