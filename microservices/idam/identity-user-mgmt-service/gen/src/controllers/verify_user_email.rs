// User-owned controller for handler 'verify_user_email'.

use crate::handlers::verify_user_email::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(VerifyUserEmailController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
