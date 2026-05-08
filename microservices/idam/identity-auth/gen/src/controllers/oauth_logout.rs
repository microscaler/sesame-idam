
// User-owned controller for handler 'oauth_logout'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::oauth_logout::{ Request, Response };



#[handler(OauthLogoutController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        
    }
    
    
}
