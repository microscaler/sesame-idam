
// Implementation stub for handler 'login_dual_otp'
// This file is a starting point for your implementation.
// You can modify this file freely - it will NOT be auto-regenerated.
// To regenerate this stub, use: brrtrouter-gen generate-stubs --path login_dual_otp --force

use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::login_dual_otp::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;



#[handler(LoginDualOtpController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    // TODO: Implement your business logic here
    // 
    // Example: Access request data
    // let email = req.inner.email;// let phone = req.inner.phone;// let send_welcome_email = req.inner.send_welcome_email;
    //
    // Example: Database query, validation, etc.
    // let result = your_service.process(&req.inner)?;
    //
    // Example: Return response
    
    Response {
        both_verified: None, // TODO: Set from your business logic
        email_sent: true, // TODO: Set from your business logic
        email_verified: None, // TODO: Set from your business logic
        message: None, // TODO: Set from your business logic
        phone_sent: true, // TODO: Set from your business logic
        phone_verified: None, // TODO: Set from your business logic
        success: true, // TODO: Set from your business logic
    }
    
}
