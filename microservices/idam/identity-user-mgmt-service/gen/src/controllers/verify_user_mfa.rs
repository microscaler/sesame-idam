// User-owned controller for handler 'verify_user_mfa'.

use crate::handlers::verify_user_mfa::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(VerifyUserMfaController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
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
        Ok(parsed) => return HttpJson::ok(parsed),
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    HttpJson::ok(Response {
        access_token: "example".to_string(),
        expires_in: 42,
        id_token: Some(Default::default()),
        refresh_token: Some(Default::default()),
        scope: Some(Default::default()),
        token_type: "example".to_string(),
    })
}
