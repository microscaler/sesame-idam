// User-owned controller for handler 'verify_dual_otp'.

use crate::handlers::verify_dual_otp::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(VerifyDualOtpController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        newly_verified_email: Some(true),
        newly_verified_phone: Some(true),
    }
}
