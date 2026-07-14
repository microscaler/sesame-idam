// BRRTRouter: user-owned

//! `GET /platform/tenants/{slug}` — tenant detail + OAuth metadata.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use lifeguard::{ColumnTrait, LifeExecutor, LifeModelTrait};
use sesame_idam_identity_login_service_gen::handlers::platform_tenant_get::Request;

use crate::models::tenant_oauth_provider::{Column as OauthColumn, Entity as OauthEntity};
use crate::services::tenant_service::TenantService;

#[handler(PlatformTenantGetController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let slug = req.data.slug.trim();
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
            tracing::error!(error = %e, %slug, "platform_tenant_get: lookup failed");
            return internal_error();
        }
    };

    let oauth_providers = match list_oauth_metadata(slug, exec) {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, %slug, "platform_tenant_get: oauth list failed");
            return internal_error();
        }
    };

    let mut body = TenantService::to_json(&tenant);
    if let Some(obj) = body.as_object_mut() {
        obj.insert(
            "oauth_providers".to_string(),
            serde_json::Value::Array(oauth_providers),
        );
    }

    HttpJson::ok(body)
}

fn list_oauth_metadata<E: LifeExecutor>(
    slug: &str,
    exec: &E,
) -> Result<Vec<serde_json::Value>, lifeguard::LifeError> {
    let rows = OauthEntity::find()
        .filter(OauthColumn::TenantSlug.eq(slug.to_string()))
        .all(exec)?;

    Ok(rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "provider": row.provider,
                "client_id": row.client_id,
                "redirect_uris": row.redirect_uris.split(',').map(str::trim).filter(|s| !s.is_empty()).collect::<Vec<_>>(),
                "secret_env_key": row.secret_env_key,
                "client_id_env_key": row.client_id_env_key,
                "config_version": row.config_version,
                "enabled": row.enabled,
                "last_rotated_at": row.last_rotated_at.map(|t| t.to_rfc3339()),
                "last_rotated_by": row.last_rotated_by,
            })
        })
        .collect())
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
