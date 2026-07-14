// BRRTRouter: user-owned

//! `PATCH /platform/tenants/{slug}/status` — lifecycle transitions.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::platform_tenant_status_patch::Request;

use crate::services::tenant_service::TenantService;

#[handler(PlatformTenantStatusPatchController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let slug = req.data.slug.trim();
    let new_status = req.data.status.trim();

    let exec = sesame_idam_database::db();

    match TenantService::transition_status(slug, new_status, exec) {
        Ok(tenant) => HttpJson::ok(TenantService::to_json(&tenant)),
        Err(e) => HttpJson::new(
            e.http_status(),
            serde_json::json!({
                "error": e.api_error(),
                "error_description": e.api_error()
            }),
        ),
    }
}
