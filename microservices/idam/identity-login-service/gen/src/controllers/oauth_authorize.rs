// User-owned controller for handler 'oauth_authorize'.

use crate::handlers::oauth_authorize::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(OauthAuthorizeController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
