// User-owned controller for handler 'login_email_otp'.

use crate::handlers::login_email_otp::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(LoginEmailOtpController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        message: Some("example".to_string()),
        success: Some(true),
    })
}
