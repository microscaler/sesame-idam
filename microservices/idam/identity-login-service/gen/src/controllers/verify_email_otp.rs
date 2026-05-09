// User-owned controller for handler 'verify_email_otp'.

use crate::handlers::verify_email_otp::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(VerifyEmailOtpController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        access_token: "example".to_string(),
        email: Some("example".to_string()),
        email_verified: Some(true),
        expires_in: 42,
        id_token: Some("example".to_string()),
        mfa_required: Some(true),
        phone_verified: Some(true),
        refresh_token: "example".to_string(),
        refresh_token_expires_in: Some(42),
        scope: Some("example".to_string()),
        token_type: "example".to_string(),
        user_id: "example".to_string(),
    }
}
