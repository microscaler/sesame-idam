// BRRTRouter: user-owned

//! `POST /platform/tenants/{slug}/oauth/{provider}/rotate` — rotation audit.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use lifeguard::LifeError;
use sesame_idam_identity_login_service_gen::handlers::platform_tenant_oauth_rotate::Request;

use crate::services::tenant_oauth_service::TenantOAuthService;

#[handler(PlatformTenantOauthRotateController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let slug = req.data.slug.trim();
    let provider = req.data.provider.trim();
    let rotated_by = req.data.rotated_by.trim();

    if rotated_by.is_empty() {
        return HttpJson::new(
            400,
            serde_json::json!({
                "error": "invalid_request",
                "error_description": "rotated_by required"
            }),
        );
    }

    let exec = sesame_idam_database::db();

    match TenantOAuthService::record_rotation(slug, provider, rotated_by, exec) {
        Ok(version) => HttpJson::ok(serde_json::json!({ "config_version": version })),
        Err(e) => map_rotate_error(e),
    }
}

fn map_rotate_error(e: LifeError) -> HttpJson<serde_json::Value> {
    let msg = e.to_string();
    if msg.contains("oauth_config_not_found") {
        return HttpJson::new(
            404,
            serde_json::json!({
                "error": "oauth_config_not_found",
                "error_description": "oauth_config_not_found"
            }),
        );
    }
    tracing::error!(error = %e, "platform_tenant_oauth_rotate failed");
    HttpJson::new(
        500,
        serde_json::json!({
            "error": "internal_error",
            "error_description": "internal_error"
        }),
    )
}
