// User-owned controller for handler 'verify_dual_otp'.

use crate::handlers::verify_dual_otp::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(VerifyDualOtpController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "access_token": "eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiIxMjMiLCJlbWFpbCI6ImFsaWNlQGV4cC5jb20ifQ.sig",
    //   "email": "alice@example.com",
    //   "email_verified": true,
    //   "expires_in": 900,
    //   "id_token": null,
    //   "mfa_required": false,
    //   "phone_verified": true,
    //   "refresh_token": "cmVmcmVzaC10b2tlbi1kdWFsLW90cA",
    //   "refresh_token_expires_in": 2592000,
    //   "scope": "openid",
    //   "token_type": "Bearer",
    //   "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "access_token": "eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiIxMjMiLCJlbWFpbCI6ImFsaWNlQGV4cC5jb20ifQ.sig",
  "email": "alice@example.com",
  "email_verified": true,
  "expires_in": 900,
  "id_token": null,
  "mfa_required": false,
  "phone_verified": true,
  "refresh_token": "cmVmcmVzaC10b2tlbi1kdWFsLW90cA",
  "refresh_token_expires_in": 2592000,
  "scope": "openid",
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
        newly_verified_email: Some(true),
        newly_verified_phone: Some(true),
    }
}
