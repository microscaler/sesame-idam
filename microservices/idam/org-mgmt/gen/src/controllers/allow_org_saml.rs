// User-owned controller for handler 'allow_org_saml'.

use crate::handlers::allow_org_saml::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(AllowOrgSamlController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
