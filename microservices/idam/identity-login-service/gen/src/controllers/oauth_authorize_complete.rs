// User-owned controller for handler 'oauth_authorize_complete'.

use crate::handlers::oauth_authorize_complete::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(OauthAuthorizeCompleteController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        error: "example".to_string(),
        error_description: Some("example".to_string()),
        hint: Some("example".to_string()),
        retry_after: Some(42),
    })
}
