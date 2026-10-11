// User-owned controller for handler 'delete_org_domain'.

use crate::handlers::delete_org_domain::Request;
use brrtrouter::typed::HttpNoContent;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(DeleteOrgDomainController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpNoContent {
    HttpNoContent
}
