
// Implementation stub for handler 'verify_user_phone'
// This file is a starting point for your implementation.
// You can modify this file freely - it will NOT be auto-regenerated.
// To regenerate this stub, use: brrtrouter-gen generate-stubs --path verify_user_phone --force

use brrtrouter_macros::handler;
use sesame_idam_identity_user_mgmt_service_gen::handlers::verify_user_phone::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;



#[handler(VerifyUserPhoneController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    // TODO: Implement your business logic here
    // 
    // Example: Access request data
    // let code = req.inner.code;// let phone_number = req.inner.phone_number;// let user_id = req.inner.user_id;
    //
    // Example: Database query, validation, etc.
    // let result = your_service.process(&req.inner)?;
    //
    // Example: Return response
    
    Response {
    }
    
}
