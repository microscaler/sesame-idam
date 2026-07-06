//! `GET /admin/users/email` — fetch a user by email (tenant-scoped).

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_user_mgmt_service_gen::handlers::fetch_user_by_email::Request;

use crate::services::user_admin_service::{user_response_json, UserAdminService};

#[handler(FetchUserByEmailController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let tenant_id = req.data.x_tenant_id.clone();
    let email = req.data.email.trim().to_lowercase();

    let exec = sesame_idam_database::db();
    match UserAdminService::find_by_email(&tenant_id, &email, exec) {
        Ok(Some(user)) => HttpJson::ok(user_response_json(&user)),
        Ok(None) => HttpJson::new(
            404,
            serde_json::json!({
                "error": "invalid_request",
                "error_description": "User not found",
            }),
        ),
        Err(e) => {
            tracing::error!(error = %e, "fetch_user_by_email: lookup failed");
            HttpJson::new(
                500,
                serde_json::json!({
                    "error": "internal_error",
                    "error_description": "An unexpected error occurred",
                }),
            )
        }
    }
}
