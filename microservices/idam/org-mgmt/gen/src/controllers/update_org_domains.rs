// User-owned controller for handler 'update_org_domains'.

use crate::handlers::update_org_domains::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(UpdateOrgDomainsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
