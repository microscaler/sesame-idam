
// User-owned controller for handler 'social_login'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::social_login::{ Request, Response };



#[handler(SocialLoginController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        
    }
    
    
}
