// User-owned controller for handler 'passkey_verify'.

use crate::handlers::passkey_verify::{ApiResponse, Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(PasskeyVerifyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> ApiResponse {
    ApiResponse::Ok(Response {
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
