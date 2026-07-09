// BRRTRouter: user-owned

//! Add existing user to organization — creates active membership.

use brrtrouter::dispatcher::{HandlerRequest, HandlerResponse};
use sesame_idam_database::db;

use crate::services::org_lifecycle::{self, OrgLifecycleError};

pub fn handle(req: HandlerRequest) -> HandlerResponse {
    let tenant_id = req
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("x-tenant-id"))
        .map(|(_, v)| v.clone())
        .unwrap_or_default();
    let org_id = req.get_path_param("org_id").unwrap_or_default();
    let user_id = req.get_path_param("user_id").unwrap_or_default();

    let body = req.body.clone().unwrap_or(serde_json::json!({}));
    let role = body
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or("member");

    if tenant_id.is_empty() || org_id.is_empty() || user_id.is_empty() {
        return HandlerResponse::json(
            400,
            serde_json::json!({
                "error": "validation_error",
                "message": "X-Tenant-ID, org_id, and user_id are required"
            }),
        );
    }

    let exec = db();
    match org_lifecycle::add_user_membership(exec, &tenant_id, &org_id, &user_id, role) {
        Ok(()) => HandlerResponse::json(200, serde_json::json!({ "success": true })),
        Err(OrgLifecycleError::NotFound) => HandlerResponse::json(
            404,
            serde_json::json!({
                "error": "not_found",
                "error_description": "Organization not found"
            }),
        ),
        Err(e) => HandlerResponse::error(500, &format!("{e:?}")),
    }
}
