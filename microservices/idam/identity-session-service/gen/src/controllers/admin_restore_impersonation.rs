// User-owned controller for handler 'admin_restore_impersonation'.

use crate::handlers::admin_restore_impersonation::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(AdminRestoreImpersonationController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        access_token: "example".to_string(),
        impersonated_user_id: "example".to_string(),
        original_user_id: "example".to_string(),
        refresh_token: "example".to_string(),
    }
}
