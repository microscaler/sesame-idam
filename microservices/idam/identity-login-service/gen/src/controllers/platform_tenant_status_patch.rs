// User-owned controller for handler 'platform_tenant_status_patch'.

use crate::handlers::platform_tenant_status_patch::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(PlatformTenantStatusPatchController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        created_at: "example".to_string(),
        display_name: "example".to_string(),
        id: "example".to_string(),
        provisioning_mode: "example".to_string(),
        slug: "example".to_string(),
        status: "example".to_string(),
        updated_at: "example".to_string(),
    })
}
