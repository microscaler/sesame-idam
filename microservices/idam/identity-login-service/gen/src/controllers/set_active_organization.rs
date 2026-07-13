// User-owned controller for handler 'set_active_organization'.

use crate::handlers::set_active_organization::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(SetActiveOrganizationController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        access_token: "example".to_string(),
        entitlements_hash: Some("example".to_string()),
        entitlements_ref: Some("example".to_string()),
        expires_in: 42,
        id_token: Some("example".to_string()),
        mfa_required: Some(true),
        permissions: Some(vec![]),
        refresh_token: "example".to_string(),
        refresh_token_expires_in: Some(42),
        roles: Some(vec![]),
        scope: Some("example".to_string()),
        token_type: "example".to_string(),
        token_version: Some(42),
        user_id: "example".to_string(),
    })
}
