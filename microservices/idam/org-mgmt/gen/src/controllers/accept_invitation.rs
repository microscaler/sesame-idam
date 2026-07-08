// User-owned controller for handler 'accept_invitation'.

use crate::handlers::accept_invitation::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(AcceptInvitationController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "error": "unauthorized",
    //   "error_description": "Authentication required"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "error": "unauthorized",
  "error_description": "Authentication required"
}"###,
    ) {
        Ok(parsed) => return parsed,
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    Response {
        error: "unauthorized".to_string(),
        error_description: Some("Authentication required".to_string()),
        hint: Some("example".to_string()),
        retry_after: Some(42),
    }
}
