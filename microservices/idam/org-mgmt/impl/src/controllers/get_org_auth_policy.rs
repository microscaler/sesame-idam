// BRRTRouter: user-owned

//! `GET /organizations/{org_id}/auth-policy` — the organisation's sign-in policy (members).

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_org_mgmt::services::auth_policy;
use sesame_idam_org_mgmt_gen::handlers::get_org_auth_policy::Request;

use super::org_domain_http::caller;

#[handler(GetOrgAuthPolicyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let (caller, tenant) = match caller(&req.jwt_claims) {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Ok(org_id) = uuid::Uuid::parse_str(&req.data.org_id) else {
        let e = auth_policy::PolicyError::NotFound;
        return HttpJson::new(e.status(), e.body());
    };
    match auth_policy::get(&tenant, org_id, caller) {
        Ok(body) => HttpJson::ok(body),
        Err(e) => {
            if let auth_policy::PolicyError::Db(detail) = &e {
                tracing::error!(error = %detail, "get_org_auth_policy failed");
            }
            HttpJson::new(e.status(), e.body())
        }
    }
}
