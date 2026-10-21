// User-owned controller for handler 'tenant_oidc_client_delete'.

use crate::handlers::tenant_oidc_client_delete::Request;
use brrtrouter::typed::HttpNoContent;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(TenantOidcClientDeleteController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpNoContent {
    HttpNoContent
}
