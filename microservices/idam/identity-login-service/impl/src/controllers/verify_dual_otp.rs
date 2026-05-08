
// Implementation stub for handler 'verify_dual_otp'
// This file is a starting point for your implementation.
// You can modify this file freely - it will NOT be auto-regenerated.
// To regenerate this stub, use: brrtrouter-gen generate-stubs --path verify_dual_otp --force

use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::verify_dual_otp::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;



#[handler(VerifyDualOtpController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    // TODO: Implement your business logic here
    // 
    // Example: Access request data
    // let email = req.inner.email;// let email_code = req.inner.email_code;// let phone = req.inner.phone;// let phone_code = req.inner.phone_code;// let session_id = req.inner.session_id;
    //
    // Example: Database query, validation, etc.
    // let result = your_service.process(&req.inner)?;
    //
    // Example: Return response
    
    Response {
        email_verified: None, // TODO: Set from your business logic
        newly_verified_email: None, // TODO: Set from your business logic
        newly_verified_phone: None, // TODO: Set from your business logic
        phone_verified: None, // TODO: Set from your business logic
    }
    
}
