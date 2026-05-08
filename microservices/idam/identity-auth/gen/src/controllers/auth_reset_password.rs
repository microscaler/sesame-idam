
// User-owned controller for handler 'auth_reset_password'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::auth_reset_password::{ Request, Response };



#[handler(AuthResetPasswordController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        
    }
    
    
}
