// User-owned controller for handler 'step_up_verify'.

use crate::handlers::step_up_verify::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(StepUpVerifyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "access_token": "eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiIxMjMiLCJsZXZlbCI6ImhpZ2gifQ.sig",
    //   "email": "alice@example.com",
    //   "email_verified": true,
    //   "expires_in": 300,
    //   "id_token": null,
    //   "mfa_required": false,
    //   "phone_verified": false,
    //   "refresh_token": "bmV3LXJlZnJlc2gtdG9rZW4tc3RlcC11cA",
    //   "refresh_token_expires_in": 3600,
    //   "scope": "openid profile",
    //   "token_type": "Bearer",
    //   "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "access_token": "eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiIxMjMiLCJsZXZlbCI6ImhpZ2gifQ.sig",
  "email": "alice@example.com",
  "email_verified": true,
  "expires_in": 300,
  "id_token": null,
  "mfa_required": false,
  "phone_verified": false,
  "refresh_token": "bmV3LXJlZnJlc2gtdG9rZW4tc3RlcC11cA",
  "refresh_token_expires_in": 3600,
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
        mfa_method: Some("example".to_string()),
        session_id: Some("example".to_string()),
        verified: true,
    }
}
