
// User-owned controller for handler 'oauth_authorize'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::oauth_authorize::{ Request, Response };



#[handler(OauthAuthorizeController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        
    }
    
    
}
