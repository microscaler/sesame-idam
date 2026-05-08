// User-owned controller for handler 'mcp_delete_agent'.

use crate::handlers::mcp_delete_agent::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(McpDeleteAgentController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
