// User-owned controller for handler 'set_principal_attribute'.

use crate::handlers::set_principal_attribute::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(SetPrincipalAttributeController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
