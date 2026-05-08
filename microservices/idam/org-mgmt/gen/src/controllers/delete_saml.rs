// User-owned controller for handler 'delete_saml'.

use crate::handlers::delete_saml::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(DeleteSamlController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
