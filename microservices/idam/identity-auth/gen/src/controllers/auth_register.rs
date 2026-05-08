
// User-owned controller for handler 'auth_register'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::auth_register::{ Request, Response };



#[handler(AuthRegisterController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        access_token: "example".to_string(),
    }
    
    
}
