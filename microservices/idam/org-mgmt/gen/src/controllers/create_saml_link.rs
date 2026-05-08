// User-owned controller for handler 'create_saml_link'.

use crate::handlers::create_saml_link::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(CreateSamlLinkController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        link: "example".to_string(),
        org_id: "example".to_string(),
    }
}
