// User-owned controller for handler 'platform_tenant_get'.

use crate::handlers::platform_tenant_get::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::TenantOAuthConfigResponse;

#[handler(PlatformTenantGetController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        created_at: "example".to_string(),
        display_name: "example".to_string(),
        id: "example".to_string(),
        oauth_providers: Some(vec![]),
        provisioning_mode: "example".to_string(),
        slug: "example".to_string(),
        status: "example".to_string(),
        updated_at: "example".to_string(),
    })
}
