// BRRTRouter: user-owned

//! `PATCH /organizations/{org_id}/domains/{domain_id}` — auto-join on or off (organisation admins).

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use chrono::Utc;
use sesame_idam_org_mgmt::services::domain_api;
use sesame_idam_org_mgmt_gen::handlers::update_org_domain::Request;

use super::org_domain_http::{caller, context, path_id, respond};

#[handler(UpdateOrgDomainController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let (caller, tenant) = match caller(&req.jwt_claims) {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let org_id = match path_id(&req.data.org_id) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let claim_id = match path_id(&req.data.domain_id) {
        Ok(id) => id,
        Err(response) => return response,
    };
    respond(
        200,
        domain_api::set_auto_join(
            context(),
            &tenant,
            org_id,
            claim_id,
            caller,
            req.data.auto_join,
            Utc::now(),
        ),
    )
}
