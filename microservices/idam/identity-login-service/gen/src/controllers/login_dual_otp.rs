// User-owned controller for handler 'login_dual_otp'.

use crate::handlers::login_dual_otp::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(LoginDualOtpController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        both_verified: Some(true),
        email_sent: true,
        email_verified: Some(true),
        message: Some("example".to_string()),
        phone_sent: true,
        phone_verified: Some(true),
        success: true,
    }
}
