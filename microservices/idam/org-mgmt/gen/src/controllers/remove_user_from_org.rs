// User-owned controller for handler 'remove_user_from_org'.

use crate::handlers::remove_user_from_org::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(RemoveUserFromOrgController)]
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
