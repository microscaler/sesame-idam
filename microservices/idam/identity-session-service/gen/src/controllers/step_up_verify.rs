// User-owned controller for handler 'step_up_verify'.

use crate::handlers::step_up_verify::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(StepUpVerifyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        mfa_method: Some("example".to_string()),
        session_id: Some("example".to_string()),
        verified: true,
    }
}
