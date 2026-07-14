//! HTTP helpers for tenant registry gate failures.

use brrtrouter::typed::HttpJson;

use super::tenant_service::TenantGateError;

/// Map tenant gate errors to REST responses (`404` for unknown slug).
#[must_use]
pub fn tenant_http_error(err: &TenantGateError) -> HttpJson<serde_json::Value> {
    let status = match err {
        TenantGateError::Unknown => 404,
        TenantGateError::NotActive => 403,
        TenantGateError::Db(_) => 500,
    };
    HttpJson::new(
        status,
        serde_json::json!({
            "error": err.api_error(),
            "error_description": err.api_error(),
        }),
    )
}
