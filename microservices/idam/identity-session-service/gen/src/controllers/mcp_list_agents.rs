// User-owned controller for handler 'mcp_list_agents'.

use crate::handlers::mcp_list_agents::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::McpAgent;

#[handler(McpListAgentsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        agents: vec![],
        has_more: true,
        page: 42,
        page_size: 42,
        total: 42,
    })
}
