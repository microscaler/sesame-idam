// User-owned controller for handler 'mcp_delete_agent'.

use crate::handlers::mcp_delete_agent::Request;
use brrtrouter::typed::HttpNoContent;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(McpDeleteAgentController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpNoContent {
    HttpNoContent
}
