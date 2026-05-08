// User-owned controller for handler 'delete_api_key'.

use crate::handlers::delete_api_key::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(DeleteApiKeyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
