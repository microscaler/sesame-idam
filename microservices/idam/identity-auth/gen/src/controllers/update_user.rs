
// User-owned controller for handler 'update_user'.

use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use crate::handlers::update_user::{ Request, Response };


#[allow(unused_imports)]
use crate::handlers::types::MfaFactor;



#[handler(UpdateUserController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    
    
    
    Response {
        can_create_orgs: Some(true),created_at: 42,email: "example".to_string(),email_confirmed: true,enabled: true,first_name: Some("example".to_string()),has_password: true,last_active_at: Some(42),last_name: Some("example".to_string()),legacy_user_id: Some("example".to_string()),locked: true,mfa_enabled: Some(true),mfa_factors: Some(vec![]),phone_number: Some("example".to_string()),phone_verified: Some(true),picture_url: Some("example".to_string()),properties: Some(Default::default()),update_password_required: Some(true),user_id: "example".to_string(),username: Some("example".to_string()),
    }
    
    
}
