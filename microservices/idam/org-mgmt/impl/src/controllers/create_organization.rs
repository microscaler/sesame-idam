//! POST /organizations — tenant consumer self-service org creation.

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

    let body = req.body.clone().unwrap_or(serde_json::json!({}));
    let name = body
        .get("name")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .unwrap_or("");

    if name.is_empty() {
        return HandlerResponse::json(
            400,
            serde_json::json!({
                "error": "validation_error",
                "message": "name is required"
            }),
        );
    }

    let exec = db();
    match org_lifecycle::create_organization(exec, &tenant_id, &user_id, name) {
        Ok(org) => HandlerResponse::json(
            201,
            serde_json::json!({
                "id": org.id.to_string(),
                "name": org.name,
                "tenant_id": org.tenant_id,
            }),
        ),
        Err(OrgLifecycleError::AlreadyHasOrganization) => HandlerResponse::json(
            409,
            serde_json::json!({
                "error": "organization_exists",
                "message": "Account already belongs to an organization"
            }),
        ),
        Err(e) => HandlerResponse::error(500, &format!("{e:?}")),
    }
}
