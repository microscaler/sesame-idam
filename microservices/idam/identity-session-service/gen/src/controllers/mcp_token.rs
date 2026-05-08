// User-owned controller for handler 'mcp_token'.

use crate::handlers::mcp_token::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(McpTokenController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        access_token: "example".to_string(),
        expires_in: Some(42),
        scope: Some("example".to_string()),
        token_type: Some("example".to_string()),
    }
}
