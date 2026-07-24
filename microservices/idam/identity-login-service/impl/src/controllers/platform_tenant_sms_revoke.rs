// BRRTRouter: user-owned

//! `DELETE /platform/tenants/{slug}/sms/{environment}` — revoke a tenant's
//! SMS sender.
//!
//! Revocation flips the status rather than deleting the row: sending stops
//! immediately (only `active` configs resolve), callers fall back to email,
//! and the revocation stays visible for audit. The sealed material is cleared
//! at the same time — a revoked credential is one we should no longer be able
//! to use even if the status check were ever bypassed.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::platform_tenant_sms_revoke::Request;

use crate::controllers::platform_tenant_sms_upsert::{internal_error, not_found, view_json};
use crate::services::tenant_sms_service::TenantSmsService;

#[handler(PlatformTenantSmsRevokeController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let slug = req.data.slug.trim();
    let environment = req.data.environment.trim();
    let exec = sesame_idam_database::db();

    match TenantSmsService::find(slug, environment, exec) {
        Ok(Some(_)) => {}
        Ok(None) => return not_found(),
        Err(e) => {
            tracing::error!(error = %e, tenant = slug, environment, "sms revoke: lookup failed");
            return internal_error();
        }
    }

    if let Err(e) = TenantSmsService::revoke(slug, environment, exec) {
        tracing::error!(error = %e, tenant = slug, environment, "sms revoke failed");
        return internal_error();
    }
    tracing::info!(tenant = slug, environment, "tenant SMS sender revoked");

    match TenantSmsService::find(slug, environment, exec) {
        Ok(Some(config)) => view_json(&config),
        Ok(None) => not_found(),
        Err(e) => {
            tracing::error!(error = %e, "sms config read-back failed");
            internal_error()
        }
    }
}
