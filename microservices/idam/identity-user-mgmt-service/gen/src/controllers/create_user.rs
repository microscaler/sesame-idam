// User-owned controller for handler 'create_user'.

use crate::handlers::create_user::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(CreateUserController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        email: "example".to_string(),
        email_confirmed: Some(true),
        enabled: true,
        first_name: "example".to_string(),
        has_password: Some(true),
        last_name: "example".to_string(),
        locked: Some(true),
        picture_url: Some(Default::default()),
        properties: Some(Default::default()),
        user_id: "example".to_string(),
        username: "example".to_string(),
    })
}
