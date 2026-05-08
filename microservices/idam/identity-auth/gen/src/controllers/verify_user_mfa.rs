
// User-owned controller for handler 'verify_user_mfa'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::verify_user_mfa::{ Request, Response };



#[handler(VerifyUserMfaController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        access_token: "example".to_string(),expires_in: 42,id_token: Some("example".to_string()),refresh_token: Some("example".to_string()),scope: Some("example".to_string()),token_type: "example".to_string(),
    }
    
    
}
