// User-owned controller for handler 'jwks'.

use crate::handlers::jwks::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(JwksController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response { keys: vec![] }
}
