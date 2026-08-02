// User-owned controller for handler 'tenant_oidc_client_delete'.

use crate::handlers::tenant_oidc_client_delete::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(TenantOidcClientDeleteController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        error: "example".to_string(),
        error_description: Some("example".to_string()),
        hint: Some("example".to_string()),
        retry_after: Some(42),
    })
}
