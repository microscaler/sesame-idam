// User-owned controller for handler 'mcp_delete_agent'.

use crate::handlers::mcp_delete_agent::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(McpDeleteAgentController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "error": "not_found",
    //   "error_description": "Agent not found"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "error": "not_found",
  "error_description": "Agent not found"
}"###,
    ) {
        Ok(parsed) => return parsed,
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    Response {
        error: "not_found".to_string(),
        error_description: Some("Agent not found".to_string()),
        hint: Some("example".to_string()),
        retry_after: Some(42),
    }
}
