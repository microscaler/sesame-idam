// User-owned controller for handler 'set_oidc_idp_metadata'.

use crate::handlers::set_oidc_idp_metadata::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(SetOidcIdpMetadataController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {})
}
