// User-owned controller for handler 'social_callback'.

use crate::handlers::social_callback::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(SocialCallbackController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "access_token": "eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiIxMjMiLCJlbWFpbCI6ImFsaWNlQGV4cC5jb20ifQ.sig",
    //   "expires_in": 900,
    //   "id_token": null,
    //   "mfa_required": false,
    //   "refresh_token": "cmVmcmVzaC10b2tlbi1zb2NpYWwtZ2l0aHVi",
    //   "refresh_token_expires_in": 2592000,
    //   "scope": "openid profile",
    //   "token_type": "Bearer",
    //   "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "access_token": "eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiIxMjMiLCJlbWFpbCI6ImFsaWNlQGV4cC5jb20ifQ.sig",
  "expires_in": 900,
  "id_token": null,
  "mfa_required": false,
  "refresh_token": "cmVmcmVzaC10b2tlbi1zb2NpYWwtZ2l0aHVi",
  "refresh_token_expires_in": 2592000,
  "scope": "openid profile",
  "token_type": "Bearer",
  "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d"
}"###,
    ) {
        Ok(parsed) => return parsed,
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    Response {
        access_token: "eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiIxMjMiLCJlbWFpbCI6ImFsaWNlQGV4cC5jb20ifQ.sig"
            .to_string(),
        expires_in: 900,
        refresh_token: "cmVmcmVzaC10b2tlbi1zb2NpYWwtZ2l0aHVi".to_string(),
        social_provider: "example".to_string(),
        social_provider_user_id: Some("example".to_string()),
        token_type: "Bearer".to_string(),
        user_id: "31c41c16-c281-44ae-9602-8a047e3bf33d".to_string(),
    }
}
