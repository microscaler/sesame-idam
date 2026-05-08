
// User-owned controller for handler 'auth_login'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::auth_login::{ Request, Response };



#[handler(AuthLoginController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        access_token: "example".to_string(),
    }
    
    
}
