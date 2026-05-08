// User-owned controller for handler 'admin_impersonate'.

use crate::handlers::admin_impersonate::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(AdminImpersonateController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        access_token: "example".to_string(),
        impersonated_user_id: "example".to_string(),
        original_user_id: "example".to_string(),
        refresh_token: "example".to_string(),
    }
}
