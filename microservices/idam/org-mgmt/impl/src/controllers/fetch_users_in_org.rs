// BRRTRouter: user-owned

//! `GET /organizations/{org_id}/users` — list members (tenant-isolated).

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_org_mgmt_gen::handlers::fetch_users_in_org::Request;

use crate::services::org_lifecycle::{self, OrgLifecycleError};
use sesame_idam_org_mgmt::org_auth;

#[handler(FetchUsersInOrgController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let (caller_id, tenant_id) =
        match org_auth::require_caller(&req.jwt_claims, &req.data.x_tenant_id) {
            Ok(principal) => principal,
            Err(response) => return response,
        };

    let page_size = req.data.page_size.unwrap_or(10);
    let page_number = req.data.page_number.unwrap_or(0);
    let role_filter = req.data.role.as_deref();

    let exec = sesame_idam_database::db();
    match org_lifecycle::list_org_members(
        exec,
        &tenant_id,
        &req.data.org_id,
        &caller_id,
        role_filter,
        page_number,
        page_size,
    ) {
        Ok(page) => {
            let items: Vec<serde_json::Value> = page
                .items
                .into_iter()
                .map(|m| {
                    serde_json::json!({
                        "user_id": m.user_id.to_string(),
                        "email": m.email,
                        "role": m.role,
                        "created_at": m.created_at.to_rfc3339(),
                    })
                })
                .collect();
            HttpJson::ok(serde_json::json!({
                "items": items,
                "page": page.page,
                "page_size": page.page_size,
                "total": page.total,
            }))
        }
        Err(OrgLifecycleError::Forbidden) => org_auth::error_json(
            403,
            "forbidden",
            "You are not a member of this organization",
        ),
        Err(OrgLifecycleError::InvalidId(msg)) => {
            org_auth::error_json(400, "validation_error", &msg)
        }
        Err(e) => {
            tracing::error!(error = %e, "fetch_users_in_org failed");
            org_auth::error_json(500, "internal_error", "An unexpected error occurred")
        }
    }
}
