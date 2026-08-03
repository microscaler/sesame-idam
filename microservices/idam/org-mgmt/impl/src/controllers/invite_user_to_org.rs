// BRRTRouter: user-owned

//! `POST /organizations/{org_id}/invitations` — invite by email (JWT tenant).

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_org_mgmt_gen::handlers::invite_user_to_org::Request;

use crate::services::org_lifecycle::{self, OrgLifecycleError};
use sesame_idam_org_mgmt::org_auth;

#[handler(InviteUserToOrgController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let (caller_id, tenant_id) =
        match org_auth::require_caller(&req.jwt_claims, req.data.x_tenant_id.as_deref()) {
            Ok(principal) => principal,
            Err(response) => return response,
        };

    if req.data.email.trim().is_empty() {
        return org_auth::error_json(400, "validation_error", "email is required");
    }

    let role = if req.data.role.trim().is_empty() {
        "member"
    } else {
        req.data.role.as_str()
    };

    let exec = sesame_idam_database::db();
    match org_lifecycle::invite_by_email_as_admin(
        exec,
        &tenant_id,
        &req.data.org_id,
        &caller_id,
        &req.data.email,
        role,
    ) {
        Ok(created) => HttpJson::ok(serde_json::json!({
            "success": true,
            "invite_id": created.invite_id.to_string(),
            "invite_token": created.invite_token,
        })),
        Err(OrgLifecycleError::Forbidden) => org_auth::error_json(
            403,
            "forbidden",
            "Insufficient permissions to invite members",
        ),
        Err(OrgLifecycleError::NotFound) => {
            org_auth::error_json(404, "not_found", "Organization not found")
        }
        Err(OrgLifecycleError::InvalidId(msg)) => {
            org_auth::error_json(400, "validation_error", &msg)
        }
        Err(e) => {
            tracing::error!(error = %e, "invite_user_to_org failed");
            org_auth::error_json(500, "internal_error", "An unexpected error occurred")
        }
    }
}
