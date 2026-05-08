// User-owned controller for handler 'verify_user_phone'.

use crate::handlers::verify_user_phone::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(VerifyUserPhoneController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
