// User-owned controller for handler 'list_applications'.

use crate::handlers::list_applications::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(ListApplicationsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
