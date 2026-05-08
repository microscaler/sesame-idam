
// Implementation stub for handler 'users_me_patch'
// This file is a starting point for your implementation.
// You can modify this file freely - it will NOT be auto-regenerated.
// To regenerate this stub, use: brrtrouter-gen generate-stubs --path users_me_patch --force

use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::users_me_patch::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;



#[handler(UsersMePatchController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    // TODO: Implement your business logic here
    // 
    // Example: Access request data
    // let first_name = req.inner.first_name;// let last_name = req.inner.last_name;// let name = req.inner.name;// let picture_url = req.inner.picture_url;// let preferred_username = req.inner.preferred_username;
    //
    // Example: Database query, validation, etc.
    // let result = your_service.process(&req.inner)?;
    //
    // Example: Return response
    
    Response {
        email: None, // TODO: Set from your business logic
        email_verified: None, // TODO: Set from your business logic
        first_name: None, // TODO: Set from your business logic
        last_name: None, // TODO: Set from your business logic
        name: None, // TODO: Set from your business logic
        org_id: None, // TODO: Set from your business logic
        org_name: None, // TODO: Set from your business logic
        phone_number: None, // TODO: Set from your business logic
        phone_verified: None, // TODO: Set from your business logic
        picture_url: None, // TODO: Set from your business logic
        preferred_username: None, // TODO: Set from your business logic
        properties: None, // TODO: Set from your business logic
        sub: None, // TODO: Set from your business logic
        updated_at: None, // TODO: Set from your business logic
        user_id: None, // TODO: Set from your business logic
        user_permissions: None, // TODO: Set from your business logic
        user_role: None, // TODO: Set from your business logic
        username: None, // TODO: Set from your business logic
    }
    
}
