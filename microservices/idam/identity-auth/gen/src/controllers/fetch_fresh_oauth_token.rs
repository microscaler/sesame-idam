
// User-owned controller for handler 'fetch_fresh_oauth_token'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::fetch_fresh_oauth_token::{ Request, Response };



#[handler(FetchFreshOauthTokenController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        access_token: Some("example".to_string()),expires_in: Some(42),refresh_token: Some("example".to_string()),scope: Some("example".to_string()),token_type: Some("example".to_string()),
    }
    
    
}
