// User-owned controller for handler 'create_saml_link'.

use crate::handlers::create_saml_link::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(CreateSamlLinkController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        link: "example".to_string(),
        org_id: "example".to_string(),
    })
}
