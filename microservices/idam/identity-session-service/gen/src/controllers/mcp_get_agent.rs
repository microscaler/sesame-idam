// User-owned controller for handler 'mcp_get_agent'.

use crate::handlers::mcp_get_agent::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(McpGetAgentController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        active: true,
        agent_id: "example".to_string(),
        created_at: "example".to_string(),
        description: Some(Default::default()),
        name: "example".to_string(),
        updated_at: "example".to_string(),
    })
}
