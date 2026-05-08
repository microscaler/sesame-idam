// User-owned controller for handler 'scim_update_user'.

use crate::handlers::scim_update_user::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(ScimUpdateUserController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        active: Some(true),
        emails: vec![],
        id: "example".to_string(),
        name: Default::default(),
        roles: Some(vec![]),
        schemas: Some(vec![]),
        user_name: "example".to_string(),
    }
}
