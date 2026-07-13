// BRRTRouter: user-owned

//! Preview an organization invitation by token (`GET /invitations/preview?token=...`).
//!
//! Returns the org name and validity so the onboarding UX can show "you've been
//! invited to X" before the user accepts. Tenant-scoped; the token is the capability.

use brrtrouter::dispatcher::{HandlerRequest, HandlerResponse};
use sesame_idam_database::db;

use crate::jwt_context;
use crate::services::org_lifecycle::{self, OrgLifecycleError};

pub fn handle(req: HandlerRequest) -> HandlerResponse {
    let Some(tenant_id) = jwt_context::tenant_from_request(&req) else {
        return HandlerResponse::json(
            400,
            serde_json::json!({
                "error": "missing_tenant",
                "message": "X-Tenant-ID header is required"
            }),
        );
    };

    let Some(token) = req
        .query_params
        .iter()
        .find(|(k, _)| k.as_ref() == "token")
        .map(|(_, v)| v.clone())
    else {
        return HandlerResponse::json(
            400,
            serde_json::json!({
                "error": "validation_error",
                "message": "token query parameter is required"
            }),
        );
    };

    let exec = db();
    match org_lifecycle::preview_invitation(exec, &tenant_id, &token) {
        Ok(preview) => HandlerResponse::json(
            200,
            serde_json::json!({
                "organization_name": preview.organization_name,
                "valid": preview.valid,
                "expired": preview.expired,
            }),
        ),
        Err(OrgLifecycleError::NotFound) => HandlerResponse::json(
            404,
            serde_json::json!({
                "error": "not_found",
                "message": "Invitation not found"
            }),
        ),
        Err(e) => HandlerResponse::error(500, &format!("{e:?}")),
    }
}
