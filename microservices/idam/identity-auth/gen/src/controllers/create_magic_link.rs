
// User-owned controller for handler 'create_magic_link'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::create_magic_link::{ Request, Response };



#[handler(CreateMagicLinkController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        
    }
    
    
}
