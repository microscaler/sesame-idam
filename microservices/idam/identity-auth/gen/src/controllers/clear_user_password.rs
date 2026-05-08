
// User-owned controller for handler 'clear_user_password'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::clear_user_password::{ Request, Response };



#[handler(ClearUserPasswordController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        
    }
    
    
}
