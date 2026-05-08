
// User-owned controller for handler 'register_mcp_agent'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::register_mcp_agent::{ Request, Response };



#[handler(RegisterMcpAgentController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        active: Some(true),agent_id: "example".to_string(),api_key_prefix: Some("example".to_string()),created_at: "example".to_string(),description: Some("example".to_string()),last_used_at: Some("example".to_string()),max_tokens_per_minute: Some(42),metadata: Some(Default::default()),name: "example".to_string(),tool_namespace: "example".to_string(),total_tokens_issued: Some(42),
    }
    
    
}
