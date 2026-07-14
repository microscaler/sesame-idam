// BRRTRouter: user-owned

//! `PUT /platform/tenants/{slug}/oauth/{provider}` — OAuth metadata upsert.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use lifeguard::LifeError;
use sesame_idam_identity_login_service_gen::handlers::platform_tenant_oauth_upsert::Request;

use crate::services::oauth::config::SupportedProvider;
use crate::services::tenant_oauth_service::TenantOAuthService;
use crate::services::tenant_service::{TenantService, STATUS_DEPROVISIONED};

#[handler(PlatformTenantOauthUpsertController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let slug = req.data.slug.trim();
    let provider = req.data.provider.trim();

    let Some(_) = SupportedProvider::parse(provider) else {
        return HttpJson::new(
            400,
            serde_json::json!({
                "error": "unsupported_provider",
                "error_description": "unsupported_provider"
            }),
        );
    };

    let exec = sesame_idam_database::db();

    let tenant = match TenantService::find_by_slug(slug, exec) {
        Ok(Some(t)) => t,
        Ok(None) => {
            return HttpJson::new(
                404,
                serde_json::json!({
                    "error": "tenant_not_found",
                    "error_description": "tenant_not_found"
                }),
            );
        }
        Err(e) => {
            tracing::error!(error = %e, "platform_tenant_oauth_upsert: tenant lookup failed");
            return internal_error();
        }
    };

    if tenant.status == STATUS_DEPROVISIONED {
        return HttpJson::new(
            409,
            serde_json::json!({
                "error": "tenant_deprovisioned",
                "error_description": "tenant_deprovisioned"
            }),
        );
    }

    let redirect_uris = req
        .data
        .redirect_uris
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(",");

    match TenantOAuthService::upsert_metadata(
        slug,
        provider,
        &req.data.client_id,
        &redirect_uris,
        &req.data.secret_env_key,
        req.data.client_id_env_key.as_deref(),
        exec,
    ) {
        Ok(_id) => match TenantOAuthService::metadata_json(slug, provider, exec) {
            Ok(Some(body)) => HttpJson::ok(body),
            Ok(None) => internal_error(),
            Err(e) => {
                tracing::error!(error = %e, "platform_tenant_oauth_upsert: metadata load failed");
                internal_error()
            }
        },
        Err(e) => map_upsert_error(e),
    }
}

fn map_upsert_error(e: LifeError) -> HttpJson<serde_json::Value> {
    let msg = e.to_string();
    if msg.contains("tenant_unknown") {
        return HttpJson::new(
            404,
            serde_json::json!({
                "error": "tenant_not_found",
                "error_description": "tenant_not_found"
            }),
        );
    }
    tracing::error!(error = %e, "platform_tenant_oauth_upsert failed");
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
