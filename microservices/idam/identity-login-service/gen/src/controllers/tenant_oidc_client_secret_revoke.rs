// User-owned controller for handler 'tenant_oidc_client_secret_revoke'.

use crate::handlers::tenant_oidc_client_secret_revoke::Request;
use brrtrouter::typed::HttpNoContent;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(TenantOidcClientSecretRevokeController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpNoContent {
    HttpNoContent
}
