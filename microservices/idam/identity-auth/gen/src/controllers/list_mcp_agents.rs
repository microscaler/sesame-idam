
// User-owned controller for handler 'list_mcp_agents'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::list_mcp_agents::{ Request, Response };


#[allow(unused_imports)]
use crate::handlers::types::McpAgent;



#[handler(ListMcpAgentsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        agents: vec![],page: 42,page_size: 42,total: 42,
    }
    
    
}
