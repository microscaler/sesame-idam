// BRRTRouter: user-owned

//! `GET /platform/tenants/{slug}/sms/{environment}` — read the SMS sender
//! configuration.
//!
//! Returns [`SmsConfigView`], which has no field capable of carrying the auth
//! token. That is deliberate and structural rather than a filtering step a
//! future edit could forget: the credential is write-only, so a stolen
//! console session cannot read it back out.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::platform_tenant_sms_get::Request;

use crate::controllers::platform_tenant_sms_upsert::{internal_error, not_found, view_json};
use crate::services::tenant_sms_service::TenantSmsService;

#[handler(PlatformTenantSmsGetController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let slug = req.data.slug.trim();
    let environment = req.data.environment.trim();
    let exec = sesame_idam_database::db();

    match TenantSmsService::find(slug, environment, exec) {
        Ok(Some(config)) => view_json(&config),
        Ok(None) => not_found(),
        Err(e) => {
            tracing::error!(error = %e, tenant = slug, environment, "sms config lookup failed");
            internal_error()
        }
    }
}
