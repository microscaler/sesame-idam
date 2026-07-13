// User-owned controller for handler 'fetch_users_in_org'.

use crate::handlers::fetch_users_in_org::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(FetchUsersInOrgController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        items: vec![],
        page: 42,
        page_size: 42,
        total: 42,
    })
}
