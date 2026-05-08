// User-owned controller for handler 'enable_saml'.

use crate::handlers::enable_saml::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(EnableSamlController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
