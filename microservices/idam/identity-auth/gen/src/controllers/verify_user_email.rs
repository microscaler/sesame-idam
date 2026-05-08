
// User-owned controller for handler 'verify_user_email'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::verify_user_email::{ Request, Response };



#[handler(VerifyUserEmailController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        
    }
    
    
}
