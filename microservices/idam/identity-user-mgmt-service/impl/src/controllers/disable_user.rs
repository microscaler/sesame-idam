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
    // A tenant administrator of their own tenant only (sesame_common::tenant_admin, ADR-011).
    let admin = match sesame_common::tenant_admin::tenant_admin(&req.jwt_claims) {
        Ok(a) => a,
        Err(r) => return r,
    };
    if let Err(r) = admin.require_same_tenant(Some(&req.data.x_tenant_id)) {
        return r;
    }
    if req.data.user_id.trim().parse::<uuid::Uuid>().ok() == Some(admin.user_id) {
        return sesame_common::tenant_admin::forbidden("You cannot disable your own account");
    }
    if let Ok(target) = req.data.user_id.trim().parse::<uuid::Uuid>() {
        if let Err(r) = crate::controllers::user_status::may_change_status_of(&admin, target) {
            return r;
        }
    }
    set_user_status(
        &admin.tenant,
        &req.data.user_id,
        crate::services::user_admin_service::STATUS_DISABLED,
        "admin_disable_user",
    )
}
