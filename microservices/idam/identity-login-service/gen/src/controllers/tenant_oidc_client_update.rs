// User-owned controller for handler 'tenant_oidc_client_update'.

use crate::handlers::tenant_oidc_client_update::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(TenantOidcClientUpdateController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        application_id: "example".to_string(),
        audiences: vec![],
        client_id: "example".to_string(),
        client_type: "example".to_string(),
        created_at: "example".to_string(),
        grants: vec![],
        pkce_s256_required: true,
        post_logout_redirect_uris: vec![],
        redirect_uris: vec![],
        response_types: vec![],
        scopes: vec![],
        status: "example".to_string(),
        tenant_id: "example".to_string(),
        token_endpoint_auth_method: "example".to_string(),
        updated_at: "example".to_string(),
    })
}
