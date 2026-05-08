// User-owned controller for handler 'magic_link_send'.

use crate::handlers::magic_link_send::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(MagicLinkSendController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        expires_in: Some(42),
        magic_link_sent: true,
    }
}
