// User-owned controller for handler 'update_user_email'.

use crate::handlers::update_user_email::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(UpdateUserEmailController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
