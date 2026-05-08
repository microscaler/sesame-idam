
// User-owned controller for handler 'fetch_employee'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::fetch_employee::{ Request, Response };



#[handler(FetchEmployeeController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        email: Some("example".to_string()),first_name: Some("example".to_string()),last_name: Some("example".to_string()),org_id_to_org_info: Some(Default::default()),picture_url: Some("example".to_string()),user_id: Some("example".to_string()),username: Some("example".to_string()),
    }
    
    
}
