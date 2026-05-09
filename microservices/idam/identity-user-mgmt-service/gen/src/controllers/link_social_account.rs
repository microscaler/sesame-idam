// User-owned controller for handler 'link_social_account'.

use crate::handlers::link_social_account::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(LinkSocialAccountController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        redirect_url: "example".to_string(),
        state: "example".to_string(),
    }
}
