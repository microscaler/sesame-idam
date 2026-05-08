// User-owned controller for handler 'signup_validate'.

use crate::handlers::signup_validate::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(SignupValidateController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        allowed: true,
        reasons: Some(vec![]),
        requires_mfa: Some(true),
    }
}
