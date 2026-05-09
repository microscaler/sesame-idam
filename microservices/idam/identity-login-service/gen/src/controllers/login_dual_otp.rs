// User-owned controller for handler 'login_dual_otp'.

use crate::handlers::login_dual_otp::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(LoginDualOtpController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "both_verified": false,
    //   "email_sent": true,
    //   "email_verified": false,
    //   "message": "Verification codes have been sent to your email and phone",
    //   "phone_sent": true,
    //   "phone_verified": false,
    //   "success": true
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "both_verified": false,
  "email_sent": true,
  "email_verified": false,
  "message": "Verification codes have been sent to your email and phone",
  "phone_sent": true,
  "phone_verified": false,
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
        both_verified: Some(false),
        email_sent: true,
        email_verified: Some(false),
        message: Some("Verification codes have been sent to your email and phone".to_string()),
        phone_sent: true,
        phone_verified: Some(false),
        success: true,
    }
}
