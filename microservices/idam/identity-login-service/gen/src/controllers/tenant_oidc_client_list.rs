// User-owned controller for handler 'tenant_oidc_client_list'.

use crate::handlers::tenant_oidc_client_list::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::OidcClientResponse;

#[handler(TenantOidcClientListController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response(vec![]))
}
