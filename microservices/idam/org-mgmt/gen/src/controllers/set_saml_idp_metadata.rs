// User-owned controller for handler 'set_saml_idp_metadata'.

use crate::handlers::set_saml_idp_metadata::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(SetSamlIdpMetadataController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
