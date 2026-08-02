// User-owned controller for handler 'tenant_oidc_client_secret_rotate'.

use crate::handlers::tenant_oidc_client_secret_rotate::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(TenantOidcClientSecretRotateController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        client_id: "example".to_string(),
        client_secret: "example".to_string(),
        created_at: "example".to_string(),
        previous_secrets_valid_until: Some(Default::default()),
        secret_id: "example".to_string(),
    })
}
