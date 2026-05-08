
// User-owned controller for handler 'verify_user_phone'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::verify_user_phone::{ Request, Response };



#[handler(VerifyUserPhoneController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        
    }
    
    
}
