// User-owned controller for handler 'oauth_logout'.

use crate::handlers::oauth_logout::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(OauthLogoutController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
