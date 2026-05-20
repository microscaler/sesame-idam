// User-owned controller for handler 'auth_login'.

use crate::handlers::auth_login::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(AuthLoginController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "access_token": "eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiIxMjMiLCJlbWFpbCI6ImFsaWNlQGV4cC5jb20iLCJvcmdfaWQiOiIxMTg5YzQ0NCJ9.sig",
    //   "expires_in": 900,
    //   "id_token": null,
    //   "mfa_required": false,
    //   "refresh_token": "cmVmcmVzaC10b2tlbi1hbGljZS1zZXNzaW9u",
    //   "refresh_token_expires_in": 2592000,
    //   "scope": "openid profile email",
    //   "token_type": "Bearer",
    //   "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "access_token": "eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiIxMjMiLCJlbWFpbCI6ImFsaWNlQGV4cC5jb20iLCJvcmdfaWQiOiIxMTg5YzQ0NCJ9.sig",
  "expires_in": 900,
  "id_token": null,
  "mfa_required": false,
  "refresh_token": "cmVmcmVzaC10b2tlbi1hbGljZS1zZXNzaW9u",
  "refresh_token_expires_in": 2592000,
  "scope": "openid profile email",
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
        access_token: "eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiIxMjMiLCJlbWFpbCI6ImFsaWNlQGV4cC5jb20iLCJvcmdfaWQiOiIxMTg5YzQ0NCJ9.sig".to_string(),entitlements_hash: Some("example".to_string()),entitlements_ref: Some("example".to_string()),expires_in: 900,id_token: Some("example".to_string()),mfa_required: Some(false),permissions: Some(vec![]),refresh_token: "cmVmcmVzaC10b2tlbi1hbGljZS1zZXNzaW9u".to_string(),refresh_token_expires_in: Some(2592000),roles: Some(vec![]),scope: Some("openid profile email".to_string()),token_type: "Bearer".to_string(),user_id: "31c41c16-c281-44ae-9602-8a047e3bf33d".to_string(),
    }
}
