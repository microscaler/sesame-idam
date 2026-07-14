// BRRTRouter: user-owned

//! `POST /organizations/{org_id}/invitations/revoke` — revoke pending invite.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_org_mgmt_gen::handlers::revoke_pending_invite::Request;

use crate::services::org_lifecycle::{self, OrgLifecycleError};
use sesame_idam_org_mgmt::org_auth;

#[handler(RevokePendingInviteController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let (caller_id, tenant_id) =
        match org_auth::require_caller(&req.jwt_claims, &req.data.x_tenant_id) {
            Ok(principal) => principal,
            Err(response) => return response,
        };

    if req.data.invite_id.trim().is_empty() {
        return org_auth::error_json(400, "validation_error", "invite_id is required");
    }

    let exec = sesame_idam_database::db();
    match org_lifecycle::revoke_invite(
        exec,
        &tenant_id,
        &req.data.org_id,
        &caller_id,
        &req.data.invite_id,
    ) {
        Ok(()) => HttpJson::new(204, serde_json::Value::Null),
        Err(OrgLifecycleError::Forbidden) => org_auth::error_json(
            403,
            "forbidden",
            "Insufficient permissions to revoke invitation",
        ),
        Err(OrgLifecycleError::NotFound) => {
            org_auth::error_json(404, "not_found", "Invitation not found")
        }
        Err(OrgLifecycleError::InvalidId(msg)) => {
            org_auth::error_json(400, "validation_error", &msg)
        }
        Err(e) => {
            tracing::error!(error = %e, "revoke_pending_invite failed");
            org_auth::error_json(500, "internal_error", "An unexpected error occurred")
        }
    }
}
