// User-owned controller for handler 'mcp_create_agent'.

use crate::handlers::mcp_create_agent::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(McpCreateAgentController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        active: true,
        agent_id: "example".to_string(),
        created_at: "example".to_string(),
        description: Some("example".to_string()),
        name: "example".to_string(),
        updated_at: "example".to_string(),
    })
}
