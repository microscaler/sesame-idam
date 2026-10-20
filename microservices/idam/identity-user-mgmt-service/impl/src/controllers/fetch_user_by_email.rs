//! `GET /admin/users/email` — fetch a user by email (tenant-scoped).

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_user_mgmt_service_gen::handlers::fetch_user_by_email::Request;

use crate::services::user_admin_service::{user_response_json, UserAdminService};

#[handler(FetchUserByEmailController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    // A tenant administrator of their own tenant only (sesame_common::tenant_admin, ADR-011), like every other
    // /admin/users endpoint. It used to take the tenant from X-Tenant-ID with no check at all, so any valid token
    // could look up any tenant's user by e-mail (20 Oct 2026).
    let admin = match sesame_common::tenant_admin::tenant_admin(&req.jwt_claims) {
        Ok(a) => a,
        Err(r) => return r,
    };
    if let Err(r) = admin.require_same_tenant(Some(&req.data.x_tenant_id)) {
        return r;
    }
    let tenant_id = admin.tenant.clone();
    let email = req.data.email.trim().to_lowercase();

    // forced RLS on sesame_idam.users: read inside the tenant's scope
    match sesame_idam_database::with_pre_auth_tenant(&tenant_id, |exec| {
        UserAdminService::find_by_email(&tenant_id, &email, exec)
    }) {
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
