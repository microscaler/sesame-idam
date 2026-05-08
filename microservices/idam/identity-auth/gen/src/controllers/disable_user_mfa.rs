
// User-owned controller for handler 'disable_user_mfa'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::disable_user_mfa::{ Request, Response };



#[handler(DisableUserMfaController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        
    }
    
    
}
