// BRRTRouter: user-owned

//! `PUT /organizations/{org_id}/auth-policy` — set the organisation's sign-in policy (owners and admins). Audited.
//! Members who lack the required method are made to enrol one at their next sign-in (identity-login-service).

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_common::audit::{AuditEventType, AuditLogEntry};
use sesame_idam_org_mgmt::services::auth_policy;
use sesame_idam_org_mgmt_gen::handlers::set_org_auth_policy::Request;

use super::org_domain_http::caller;
use crate::audit::EMITTER;

#[handler(SetOrgAuthPolicyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let (caller, tenant) = match caller(&req.jwt_claims) {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Ok(org_id) = uuid::Uuid::parse_str(&req.data.org_id) else {
        let e = auth_policy::PolicyError::NotFound;
        return HttpJson::new(e.status(), e.body());
    };
    match auth_policy::set(&tenant, org_id, caller, &req.data.require) {
        Ok(body) => {
            match AuditLogEntry::new(AuditEventType::AuthenticatorChanged, "org-mgmt")
                .tenant_id(tenant.clone())
                .user_id(caller.to_string())
                .decision_source("org_auth_policy")
                .result("allowed")
                .reason(format!("org {org_id} require {}", req.data.require.trim()))
                .build()
            {
                Ok(entry) => EMITTER.emit(entry),
                Err(e) => tracing::warn!(error = %e, "org auth policy audit build failed"),
            }
            HttpJson::ok(body)
        }
        Err(e) => {
            if let auth_policy::PolicyError::Db(detail) = &e {
                tracing::error!(error = %detail, "set_org_auth_policy failed");
            }
            HttpJson::new(e.status(), e.body())
        }
    }
}
