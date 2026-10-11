// BRRTRouter: user-owned

//! `POST /tenants/current/org-domains/{domain_id}/revoke` — a tenant admin revokes an organisation's claim. The tenant
//! is the token's; a claim of another tenant is not found.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use chrono::Utc;
use sesame_idam_org_mgmt::org_auth;
use sesame_idam_org_mgmt::services::domain_api;
use sesame_idam_org_mgmt_gen::handlers::revoke_org_domain::Request;

use super::org_domain_http::{context, path_id, respond};

#[handler(RevokeOrgDomainController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let tenant = match domain_api::tenant_admin_tenant(req.jwt_claims.as_ref()) {
        Ok(tenant) => tenant,
        Err(401) => return org_auth::error_json(401, "unauthorized", "Authentication required"),
        Err(_) => {
            return org_auth::error_json(403, "forbidden", "Tenant administrator role required")
        }
    };
    let claim_id = match path_id(&req.data.domain_id) {
        Ok(id) => id,
        Err(response) => return response,
    };
    respond(
        200,
        domain_api::revoke(context(), &tenant, claim_id, &req.data.reason, Utc::now()),
    )
}
