// User-owned controller for handler 'platform_tenant_sms_get'.

use crate::handlers::platform_tenant_sms_get::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(PlatformTenantSmsGetController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        account_sid: Some("example".to_string()),
        campaign_ref: Some("example".to_string()),
        connected_account_sid: Some("example".to_string()),
        credential_configured: true,
        custody_mode: "example".to_string(),
        daily_spend_ceiling_cents: 42,
        environment: "example".to_string(),
        from_number: Some("example".to_string()),
        last_validated_at: Some("example".to_string()),
        messaging_service_sid: Some("example".to_string()),
        provider: "example".to_string(),
        status: "example".to_string(),
        tenant_id: "example".to_string(),
    })
}
