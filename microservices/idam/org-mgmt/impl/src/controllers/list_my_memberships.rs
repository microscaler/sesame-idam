// BRRTRouter: user-owned

//! GET /users/me/memberships — list org memberships for authenticated user.

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

    let exec = db();
    match org_lifecycle::list_memberships(exec, &tenant_id, &user_id) {
        Ok(items) => {
            let json: Vec<_> = items
                .into_iter()
                .map(|m| {
                    serde_json::json!({
                        "organization_id": m.org_id.to_string(),
                        "organization_name": m.org_name,
                        "role": m.role,
                        "status": m.status,
                    })
                })
                .collect();
            HandlerResponse::json(200, serde_json::Value::Array(json))
        }
        Err(OrgLifecycleError::InvalidId(msg)) => HandlerResponse::error(400, &msg),
        Err(e) => HandlerResponse::error(500, &format!("{e:?}")),
    }
}
