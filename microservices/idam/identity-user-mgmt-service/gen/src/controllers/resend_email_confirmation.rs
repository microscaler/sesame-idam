// User-owned controller for handler 'resend_email_confirmation'.

use crate::handlers::resend_email_confirmation::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(ResendEmailConfirmationController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "error": "invalid_request",
    //   "error_description": "Not found"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "error": "invalid_request",
  "error_description": "Not found"
}"###,
    ) {
        Ok(parsed) => return parsed,
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    Response {
        error: "invalid_request".to_string(),
        error_description: Some("Not found".to_string()),
        hint: Some("example".to_string()),
        retry_after: Some(42),
    }
}
