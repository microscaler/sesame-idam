
// User-owned controller for handler 'query_users'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::query_users::{ Request, Response };


#[allow(unused_imports)]
use crate::handlers::types::User;



#[handler(QueryUsersController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        current_page: Some(42),has_more_results: Some(true),page_size: Some(42),total_users: Some(42),users: Some(vec![]),
    }
    
    
}
