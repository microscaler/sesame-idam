
// User-owned controller for handler 'auth_token'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::auth_token::{ Request, Response };



#[handler(AuthTokenController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        access_token: "example".to_string(),expires_in: 42,id_token: Some("example".to_string()),refresh_token: Some("example".to_string()),scope: Some("example".to_string()),token_type: "example".to_string(),
    }
    
    
}
