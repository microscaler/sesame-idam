// BRRTRouter: user-owned

//! `POST /organizations/{org_id}/domains` — claim a domain (organisation admins).

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use chrono::Utc;
use sesame_idam_org_mgmt::services::domain_api;
use sesame_idam_org_mgmt_gen::handlers::create_org_domain::Request;

use super::org_domain_http::{caller, context, path_id, respond};

#[handler(CreateOrgDomainController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let (caller, tenant) = match caller(&req.jwt_claims) {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let org_id = match path_id(&req.data.org_id) {
        Ok(id) => id,
        Err(response) => return response,
    };
    respond(
        201,
        domain_api::create(
            context(),
            &tenant,
            org_id,
            caller,
            &req.data.domain,
            req.data.scope.as_deref(),
            Utc::now(),
        ),
    )
}
