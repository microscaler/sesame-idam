// BRRTRouter: user-owned

//! `GET /tenant/sms/{environment}` — a tenant reads its own SMS sender.
//!
//! Tenant self-service (ADR-011). Note what the signature does NOT contain: a
//! tenant identifier. The tenant is resolved from the bearer token, so there is
//! nothing here for a caller to point at somebody else's row.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::tenant_sms_get::Request;

use crate::controllers::platform_tenant_sms_upsert::{internal_error, not_found, view_json};
use crate::services::tenant_admin::tenant_admin_principal;
use crate::services::tenant_sms_service::TenantSmsService;

#[handler(TenantSmsGetController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let admin = match tenant_admin_principal(&req.jwt_claims) {
        Ok(a) => a,
        Err(resp) => return resp,
    };
    let environment = req.data.environment.trim();
    let exec = sesame_idam_database::db();

    match TenantSmsService::find(&admin.tenant, environment, exec) {
        Ok(Some(config)) => view_json(&config),
        Ok(None) => not_found(),
        Err(e) => {
            tracing::error!(
                error = %e, tenant = %admin.tenant, environment,
                "tenant sms config lookup failed"
            );
            internal_error()
        }
    }
}
