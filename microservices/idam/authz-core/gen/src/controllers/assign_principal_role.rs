// User-owned controller for handler 'assign_principal_role'.

use crate::handlers::assign_principal_role::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(AssignPrincipalRoleController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "error": "invalid_request",
    //   "error_description": "Bad request (validation error)"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "error": "invalid_request",
  "error_description": "Bad request (validation error)"
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
        error_description: Some("Bad request (validation error)".to_string()),
    }
}
