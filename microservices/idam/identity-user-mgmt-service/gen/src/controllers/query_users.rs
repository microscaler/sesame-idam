// User-owned controller for handler 'query_users'.

use crate::handlers::query_users::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::UserQueryItem;

#[handler(QueryUsersController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        has_more: Some(true),
        limit: Some(42),
        page: Some(42),
        total: Some(42),
        users: Some(vec![]),
    }
}
