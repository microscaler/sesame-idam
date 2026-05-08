
// User-owned controller for handler 'fetch_user_oauth_tokens'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::fetch_user_oauth_tokens::{ Request, Response };



#[handler(FetchUserOauthTokensController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        tokens: Some(vec![]),
    }
    
    
}
