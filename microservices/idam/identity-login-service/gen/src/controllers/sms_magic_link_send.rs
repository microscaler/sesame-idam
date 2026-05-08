// User-owned controller for handler 'sms_magic_link_send'.

use crate::handlers::sms_magic_link_send::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(SmsMagicLinkSendController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        expires_in: Some(42),
        magic_link_sent: true,
    }
}
