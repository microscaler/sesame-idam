//! `POST /admin/users/{user_id}/enable` — unblock a user (status=active).

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_user_mgmt_service_gen::handlers::enable_user::Request;

use crate::controllers::user_status::set_user_status;

#[handler(EnableUserController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    set_user_status(
        &req.data.x_tenant_id,
        &req.data.user_id,
        crate::services::user_admin_service::STATUS_ACTIVE,
        "admin_enable_user",
    )
}
