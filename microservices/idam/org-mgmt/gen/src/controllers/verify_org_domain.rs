// User-owned controller for handler 'verify_org_domain'.

use crate::handlers::verify_org_domain::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::OrgDomain;

#[handler(VerifyOrgDomainController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        claim: Default::default(),
        verdict: "example".to_string(),
    })
}
