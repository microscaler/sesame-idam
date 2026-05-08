// User-owned controller for handler 'mcp_get_agent'.

use crate::handlers::mcp_get_agent::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(McpGetAgentController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
