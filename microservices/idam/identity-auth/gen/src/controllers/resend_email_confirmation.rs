
// User-owned controller for handler 'resend_email_confirmation'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::resend_email_confirmation::{ Request, Response };



#[handler(ResendEmailConfirmationController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        
    }
    
    
}
