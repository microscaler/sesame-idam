// User-owned controller for handler 'principal_effective'.

use crate::handlers::principal_effective::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(PrincipalEffectiveController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        attributes: Some(Default::default()),
        permissions: vec![],
        roles: vec![],
        user_id: "example".to_string(),
    }
}
