//! `POST /admin/users/{user_id}/disable` — block a user (status=disabled).
//!
//! Disabled users fail login with the same indistinguishable 401 as bad
//! credentials (no enumeration).

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_user_mgmt_service_gen::handlers::disable_user::Request;

use crate::controllers::user_status::set_user_status;

#[handler(DisableUserController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    set_user_status(
        &req.data.x_tenant_id,
        &req.data.user_id,
        crate::services::user_admin_service::STATUS_DISABLED,
        "admin_disable_user",
    )
}
