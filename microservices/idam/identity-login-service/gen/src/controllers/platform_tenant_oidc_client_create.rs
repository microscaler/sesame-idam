// User-owned controller for handler 'platform_tenant_oidc_client_create'.

use crate::handlers::platform_tenant_oidc_client_create::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::OidcClientResponse;

#[handler(PlatformTenantOidcClientCreateController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        client: Default::default(),
        client_secret: Default::default(),
        secret_id: Default::default(),
    })
}
