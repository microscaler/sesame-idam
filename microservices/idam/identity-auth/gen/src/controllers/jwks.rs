
// User-owned controller for handler 'jwks'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::jwks::{ Request, Response };



#[handler(JwksController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        keys: vec![],
    }
    
    
}
