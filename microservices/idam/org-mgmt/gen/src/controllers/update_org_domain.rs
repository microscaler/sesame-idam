// User-owned controller for handler 'update_org_domain'.

use crate::handlers::update_org_domain::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::OrgDomainRecord;

#[handler(UpdateOrgDomainController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        auto_join: true,
        created_at: "example".to_string(),
        dnssec: true,
        domain: "example".to_string(),
        grace_until: Some("example".to_string()),
        id: "example".to_string(),
        last_checked_at: Some("example".to_string()),
        last_verified_at: Some("example".to_string()),
        org_id: "example".to_string(),
        pending_until: Some("example".to_string()),
        record: Some(Default::default()),
        scope: "example".to_string(),
        state: "example".to_string(),
        verified_at: Some("example".to_string()),
    })
}
