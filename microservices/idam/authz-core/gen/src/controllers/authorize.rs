// User-owned controller for handler 'authorize'.

use crate::handlers::authorize::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(AuthorizeController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        allowed: true,
        permissions_used: Some(vec![]),
        reason: Some("example".to_string()),
        roles_matched: Some(vec![]),
    }
}
