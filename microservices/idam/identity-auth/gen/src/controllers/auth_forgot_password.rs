
// User-owned controller for handler 'auth_forgot_password'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::auth_forgot_password::{ Request, Response };



#[handler(AuthForgotPasswordController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        
    }
    
    
}
