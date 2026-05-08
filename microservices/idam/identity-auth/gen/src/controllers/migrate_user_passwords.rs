
// User-owned controller for handler 'migrate_user_passwords'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::migrate_user_passwords::{ Request, Response };



#[handler(MigrateUserPasswordsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        
    }
    
    
}
