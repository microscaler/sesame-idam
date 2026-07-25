// BRRTRouter: user-owned

//! `DELETE /tenant/sms/{environment}` — a tenant revokes its own SMS sender.
//!
//! Tenant self-service (ADR-011); the tenant comes from the bearer token.
//!
//! Revocation flips the status rather than deleting the row, so the decision
//! stays auditable, and clears the sealed material so a revoked credential
//! cannot be used even if the status check were ever bypassed.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::tenant_sms_revoke::Request;

use crate::controllers::platform_tenant_sms_upsert::{internal_error, not_found, read_back};
use crate::services::tenant_admin::tenant_admin_principal;
use crate::services::tenant_sms_service::TenantSmsService;

#[handler(TenantSmsRevokeController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let admin = match tenant_admin_principal(&req.jwt_claims) {
        Ok(a) => a,
        Err(resp) => return resp,
    };
    let environment = req.data.environment.trim();
    let exec = sesame_idam_database::db();

    match TenantSmsService::find(&admin.tenant, environment, exec) {
        Ok(Some(_)) => {}
        Ok(None) => return not_found(),
        Err(e) => {
            tracing::error!(error = %e, tenant = %admin.tenant, environment, "sms revoke: lookup failed");
            return internal_error();
        }
    }

    if let Err(e) = TenantSmsService::revoke(&admin.tenant, environment, exec) {
        tracing::error!(error = %e, tenant = %admin.tenant, environment, "sms revoke failed");
        return internal_error();
    }
    tracing::info!(
        tenant = %admin.tenant, environment, actor = %admin.user_id,
        "tenant revoked its SMS sender"
    );

    read_back(&admin.tenant, environment, exec)
}
