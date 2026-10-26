// User-owned controller for handler 'mfa_totp_enroll'.

use crate::handlers::mfa_totp_enroll::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(MfaTotpEnrollController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        digits: 42,
        factor_id: "example".to_string(),
        otpauth_uri: "example".to_string(),
        period: 42,
        secret: "example".to_string(),
    })
}
