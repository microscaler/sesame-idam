
// User-owned controller for handler 'mcp_validate_token'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::mcp_validate_token::{ Request, Response };


#[allow(unused_imports)]
use crate::handlers::types::McpAgent;



#[handler(McpValidateTokenController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        agent: Default::default(),expires_at: Some("example".to_string()),permissions: Some(vec![]),valid: true,
    }
    
    
}
