// BRRTRouter: user-owned

//! `POST /platform/tenants` — platform ops tenant mint.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use lifeguard::LifeError;
use sesame_idam_identity_login_service_gen::handlers::platform_tenant_create::Request;

use crate::services::tenant_service::{
    TenantService, PROVISIONING_PLATFORM, PROVISIONING_SELF_SERVICE, STATUS_ACTIVE,
    STATUS_PROVISIONING,
};

#[handler(PlatformTenantCreateController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    if let Some(mode) = req.data.provisioning_mode.as_deref() {
        if mode == PROVISIONING_SELF_SERVICE {
            return HttpJson::new(
                400,
                serde_json::json!({
                    "error": "invalid_provisioning_mode",
                    "error_description": "use POST /platform/tenants/provision for self_service"
                }),
            );
        }
    }

    let slug = match TenantService::validate_slug(&req.data.slug) {
        Ok(s) => s,
        Err(e) => {
            return HttpJson::new(
                400,
                serde_json::json!({
                    "error": e.api_error(),
                    "error_description": e.api_error()
                }),
            );
        }
    };

    let display_name = req.data.display_name.trim();
    if display_name.is_empty() {
        return HttpJson::new(
            400,
            serde_json::json!({
                "error": "invalid_request",
                "error_description": "display_name required"
            }),
        );
    }

    let activate = req.data.activate.unwrap_or(true);
    let status = if activate {
        STATUS_ACTIVE
    } else {
        STATUS_PROVISIONING
    };

    let exec = sesame_idam_database::db();

    if let Ok(Some(_)) = TenantService::find_by_slug(&slug, exec) {
        return HttpJson::new(
            409,
            serde_json::json!({
                "error": "slug_taken",
                "error_description": "slug_taken"
            }),
        );
    }

    match TenantService::create(
        &slug,
        display_name,
        PROVISIONING_PLATFORM,
        status,
        exec,
    ) {
        Ok(id) => match TenantService::find_by_slug(&slug, exec) {
            Ok(Some(tenant)) => HttpJson::new(201, TenantService::to_json(&tenant)),
            Ok(None) => internal_error(),
            Err(e) => {
                tracing::error!(error = %e, %id, "platform_tenant_create: reload failed");
                internal_error()
            }
        },
        Err(e) => map_create_error(e),
    }
}

fn map_create_error(e: LifeError) -> HttpJson<serde_json::Value> {
    let msg = e.to_string();
    if msg.contains("unique") || msg.contains("duplicate") {
        return HttpJson::new(
            409,
            serde_json::json!({
                "error": "slug_taken",
                "error_description": "slug_taken"
            }),
        );
    }
    tracing::error!(error = %e, "platform_tenant_create: insert failed");
    internal_error()
}

fn internal_error() -> HttpJson<serde_json::Value> {
    HttpJson::new(
        500,
        serde_json::json!({
            "error": "internal_error",
            "error_description": "internal_error"
        }),
    )
}
