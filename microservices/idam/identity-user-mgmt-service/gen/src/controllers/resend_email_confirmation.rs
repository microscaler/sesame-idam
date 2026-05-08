// User-owned controller for handler 'resend_email_confirmation'.

use crate::handlers::resend_email_confirmation::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(ResendEmailConfirmationController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
