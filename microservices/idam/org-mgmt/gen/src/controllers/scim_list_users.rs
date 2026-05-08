// User-owned controller for handler 'scim_list_users'.

use crate::handlers::scim_list_users::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::ScimUser;

#[handler(ScimListUsersController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        resources: vec![],
        items_per_page: 42,
        schemas: Some(vec![]),
        start_index: 42,
        total_results: 42,
    }
}
