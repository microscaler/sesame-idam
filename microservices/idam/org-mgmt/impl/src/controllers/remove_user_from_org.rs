// BRRTRouter: user-owned

//! `DELETE /organizations/{org_id}/users/{user_id}` — remove member from org.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_common::VersionStore;
use sesame_idam_org_mgmt_gen::handlers::remove_user_from_org::Request;

use crate::services::org_lifecycle::{self, OrgLifecycleError};
use sesame_idam_org_mgmt::org_auth;

#[handler(RemoveUserFromOrgController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let (caller_id, tenant_id) =
        match org_auth::require_caller(&req.jwt_claims, &req.data.x_tenant_id) {
            Ok(principal) => principal,
            Err(response) => return response,
        };

    if let Err(error) =
        VersionStore::from_env().and_then(|store| store.increment_subject(&req.data.user_id))
    {
        tracing::error!(%error, user_id = %req.data.user_id, "token version bump failed");
        return org_auth::error_json(
            503,
            "security_state_unavailable",
            "Session invalidation is temporarily unavailable",
        );
    }

    let exec = sesame_idam_database::db();
    match org_lifecycle::remove_member(
        exec,
        &tenant_id,
        &req.data.org_id,
        &caller_id,
        &req.data.user_id,
    ) {
        Ok(()) => HttpJson::new(204, serde_json::Value::Null),
        Err(OrgLifecycleError::Forbidden) => org_auth::error_json(
            403,
            "forbidden",
            "Insufficient permissions to remove member",
        ),
        Err(OrgLifecycleError::NotFound) => {
            org_auth::error_json(404, "not_found", "Member not found")
        }
        Err(OrgLifecycleError::InvalidId(msg)) => {
            org_auth::error_json(400, "validation_error", &msg)
        }
        Err(e) => {
            tracing::error!(error = %e, "remove_user_from_org failed");
            org_auth::error_json(500, "internal_error", "An unexpected error occurred")
        }
    }
}
