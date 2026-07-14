// BRRTRouter: user-owned

//! `PATCH /organizations/{org_id}/users/{user_id}/role` — change member role.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_common::VersionStore;
use sesame_idam_org_mgmt_gen::handlers::change_user_role_in_org::Request;

use sesame_idam_org_mgmt::org_auth;
use crate::services::org_lifecycle::{self, OrgLifecycleError};

#[handler(ChangeUserRoleInOrgController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let (caller_id, tenant_id) =
        match org_auth::require_caller(&req.jwt_claims, &req.data.x_tenant_id) {
            Ok(principal) => principal,
            Err(response) => return response,
        };

    let primary_role = req
        .data
        .primary_role
        .as_deref()
        .unwrap_or("")
        .trim();
    if primary_role.is_empty() {
        return org_auth::error_json(400, "validation_error", "primary_role is required");
    }

    if let Err(error) = VersionStore::from_env().and_then(|store| store.increment_subject(&req.data.user_id)) {
        tracing::error!(%error, user_id = %req.data.user_id, "token version bump failed");
        return org_auth::error_json(
            503,
            "security_state_unavailable",
            "Session invalidation is temporarily unavailable",
        );
    }

    let exec = sesame_idam_database::db();
    match org_lifecycle::change_member_role(
        exec,
        &tenant_id,
        &req.data.org_id,
        &caller_id,
        &req.data.user_id,
        primary_role,
    ) {
        Ok(()) => HttpJson::new(200, serde_json::json!({})),
        Err(OrgLifecycleError::Forbidden) => org_auth::error_json(
            403,
            "forbidden",
            "Insufficient permissions to change member role",
        ),
        Err(OrgLifecycleError::NotFound) => {
            org_auth::error_json(404, "not_found", "Member not found")
        }
        Err(OrgLifecycleError::InvalidId(msg)) => {
            org_auth::error_json(400, "validation_error", &msg)
        }
        Err(e) => {
            tracing::error!(error = %e, "change_user_role_in_org failed");
            org_auth::error_json(500, "internal_error", "An unexpected error occurred")
        }
    }
}
