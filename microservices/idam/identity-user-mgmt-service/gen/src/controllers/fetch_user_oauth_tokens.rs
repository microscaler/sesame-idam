// User-owned controller for handler 'fetch_user_oauth_tokens'.

use crate::handlers::fetch_user_oauth_tokens::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(FetchUserOauthTokensController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        tokens: Some(vec![]),
    }
}
