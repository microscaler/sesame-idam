
// User-owned controller for handler 'setup_user_mfa_totp'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::setup_user_mfa_totp::{ Request, Response };



#[handler(SetupUserMfaTotpController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        provisioning_uri: Some("example".to_string()),secret: Some("example".to_string()),user_id: Some("example".to_string()),
    }
    
    
}
