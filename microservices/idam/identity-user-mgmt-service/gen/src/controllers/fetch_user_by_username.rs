// User-owned controller for handler 'fetch_user_by_username'.

use crate::handlers::fetch_user_by_username::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(FetchUserByUsernameController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        email: Some("example".to_string()),
        email_confirmed: Some(true),
        enabled: Some(true),
        first_name: Some("example".to_string()),
        has_password: Some(true),
        last_name: Some("example".to_string()),
        locked: Some(true),
        picture_url: Some("example".to_string()),
        properties: Some(Default::default()),
        user_id: Some("example".to_string()),
        username: Some("example".to_string()),
    }
}
