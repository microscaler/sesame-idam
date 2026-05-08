// User-owned controller for handler 'disable_user_mfa'.

use crate::handlers::disable_user_mfa::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(DisableUserMfaController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
