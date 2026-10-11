// BRRTRouter: user-owned

//! `GET /organizations/{org_id}/domains` — the organisation's claims (members).

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_org_mgmt::services::domain_api;
use sesame_idam_org_mgmt_gen::handlers::list_org_domains::Request;

use super::org_domain_http::{caller, context, path_id, respond};

#[handler(ListOrgDomainsController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let (caller, tenant) = match caller(&req.jwt_claims) {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let org_id = match path_id(&req.data.org_id) {
        Ok(id) => id,
        Err(response) => return response,
    };
    respond(200, domain_api::list(context(), &tenant, org_id, caller))
}
