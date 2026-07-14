// User-owned controller for handler 'platform_tenant_oauth_rotate'.

use crate::handlers::platform_tenant_oauth_rotate::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(PlatformTenantOauthRotateController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response { config_version: 42 })
}
