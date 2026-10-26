// User-owned controller for handler 'create_user'.

use crate::handlers::create_user::{ApiResponse, Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(CreateUserController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> ApiResponse {
    ApiResponse::Ok(Response {
        email: "example".to_string(),
        email_confirmed: Some(true),
        enabled: true,
        first_name: Some("example".to_string()),
        has_password: Some(true),
        last_name: Some("example".to_string()),
        locked: Some(true),
        picture_url: Some("example".to_string()),
        properties: Some(Default::default()),
        user_id: "example".to_string(),
        username: Some("example".to_string()),
    })
}
