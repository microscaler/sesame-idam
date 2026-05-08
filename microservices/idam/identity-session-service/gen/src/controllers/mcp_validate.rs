// User-owned controller for handler 'mcp_validate'.

use crate::handlers::mcp_validate::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(McpValidateController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        agent_id: Some("example".to_string()),
        expires_at: Some("example".to_string()),
        permissions: Some(vec![]),
        valid: true,
    }
}
