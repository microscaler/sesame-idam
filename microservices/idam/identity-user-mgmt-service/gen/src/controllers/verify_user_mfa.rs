// User-owned controller for handler 'verify_user_mfa'.

use crate::handlers::verify_user_mfa::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(VerifyUserMfaController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "message": "MFA verified successfully",
    //   "mfa_required": false,
    //   "success": true
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "message": "MFA verified successfully",
  "mfa_required": false,
  "success": true
}"###,
    ) {
        Ok(parsed) => return parsed,
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    Response {
        access_token: "example".to_string(),
        expires_in: 42,
        id_token: Some("example".to_string()),
        refresh_token: Some("example".to_string()),
        scope: Some("example".to_string()),
        token_type: "example".to_string(),
    }
}
