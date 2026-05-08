// User-owned controller for handler 'disallow_org_saml'.

use crate::handlers::disallow_org_saml::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(DisallowOrgSamlController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
