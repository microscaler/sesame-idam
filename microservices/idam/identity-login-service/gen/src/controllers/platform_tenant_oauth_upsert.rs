// User-owned controller for handler 'platform_tenant_oauth_upsert'.

use crate::handlers::platform_tenant_oauth_upsert::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(PlatformTenantOauthUpsertController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        client_id: "example".to_string(),
        client_id_env_key: Some("example".to_string()),
        config_version: 42,
        enabled: true,
        last_rotated_at: Some("example".to_string()),
        last_rotated_by: Some("example".to_string()),
        provider: "example".to_string(),
        redirect_uris: vec![],
        secret_env_key: "example".to_string(),
    })
}
