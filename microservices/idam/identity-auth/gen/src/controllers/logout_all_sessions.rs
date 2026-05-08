
// User-owned controller for handler 'logout_all_sessions'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::logout_all_sessions::{ Request, Response };



#[handler(LogoutAllSessionsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        
    }
    
    
}
