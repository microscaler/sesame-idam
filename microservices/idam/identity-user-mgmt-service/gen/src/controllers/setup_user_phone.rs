// User-owned controller for handler 'setup_user_phone'.

use crate::handlers::setup_user_phone::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(SetupUserPhoneController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
