// User-owned controller for handler 'admin_jwks_revoke'.

use crate::handlers::admin_jwks_revoke::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(AdminJwksRevokeController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        kid: Some("example".to_string()),
        message: Some("example".to_string()),
        success: Some(true),
    }
}
