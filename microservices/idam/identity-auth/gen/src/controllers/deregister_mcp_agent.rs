
// User-owned controller for handler 'deregister_mcp_agent'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::deregister_mcp_agent::{ Request, Response };



#[handler(DeregisterMcpAgentController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        
    }
    
    
}
