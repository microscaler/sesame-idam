// BRRTRouter: user-owned

//! Fetch org metadata for an authenticated member (`GET /organizations/{org_id}`).

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

    let Some(user_id) = jwt_context::user_id_from_request(&req) else {
        return HandlerResponse::json(
            401,
            serde_json::json!({
                "error": "unauthorized",
                "message": "Authentication required"
            }),
        );
    };

    let Some(org_id) = req
        .path_params
        .iter()
        .find(|(k, _)| k.as_ref() == "org_id")
        .map(|(_, v)| v.clone())
    else {
        return HandlerResponse::json(
            400,
            serde_json::json!({
                "error": "validation_error",
                "message": "org_id path parameter is required"
            }),
        );
    };

    let exec = db();
    match org_lifecycle::get_organization(exec, &tenant_id, &org_id, &user_id) {
        Ok(org) => HandlerResponse::json(
            200,
            serde_json::json!({
                "id": org.id.to_string(),
                "name": org.name,
                "tenant_id": org.tenant_id,
                "status": org.status,
                "metadata": org.metadata,
                "created_at": org.created_at.to_rfc3339(),
                "updated_at": org.updated_at.to_rfc3339(),
            }),
        ),
        Err(OrgLifecycleError::Forbidden) => HandlerResponse::json(
            403,
            serde_json::json!({
                "error": "forbidden",
                "message": "You are not a member of this organization"
            }),
        ),
        Err(OrgLifecycleError::NotFound) => HandlerResponse::json(
            404,
            serde_json::json!({
                "error": "not_found",
                "message": "Organization not found"
            }),
        ),
        Err(OrgLifecycleError::InvalidId(msg)) => HandlerResponse::json(
            400,
            serde_json::json!({
                "error": "validation_error",
                "message": msg
            }),
        ),
        Err(e) => HandlerResponse::error(500, &format!("{e:?}")),
    }
}
