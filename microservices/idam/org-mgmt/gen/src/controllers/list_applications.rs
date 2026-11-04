// User-owned controller for handler 'list_applications'.

use crate::handlers::list_applications::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::Application;

#[handler(ListApplicationsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        has_more: true,
        items: vec![],
        page: 42,
        page_size: 42,
        total: 42,
    })
}
