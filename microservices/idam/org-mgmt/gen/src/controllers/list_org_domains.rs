// User-owned controller for handler 'list_org_domains'.

use crate::handlers::list_org_domains::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::OrgDomain;

#[handler(ListOrgDomainsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response(vec![]))
}
