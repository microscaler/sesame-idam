// BRRTRouter: user-owned

//! `POST /admin/users/{user_id}/mfa/disable` — support-mediated recovery (ADR-008 §6): a tenant administrator
//! removes every second factor, passkey and recovery code of a user who has lost them all, so the user can sign in
//! with their password (or email) and enrol again. If their organisation requires a second factor they are made to
//! enrol one at that sign-in.
//!
//! This is the recovery path of last resort and the most abusable one, so: tenant administrators of the user's own
//! tenant only, never on their own account (they use the security page), only over users whose administrator roles
//! they could grant (as for disabling an account), always audited, and the user is emailed.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use lifeguard::LifeExecutor;
use sesame_common::audit::{AuditEventType, AuditLogEntry};
use sesame_idam_identity_user_mgmt_service_gen::handlers::disable_user_mfa::Request;

use crate::audit::EMITTER;
use crate::services::user_admin_service::UserAdminService;

#[handler(DisableUserMfaController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let admin = match sesame_common::tenant_admin::tenant_admin(&req.jwt_claims) {
        Ok(a) => a,
        Err(r) => return r,
    };
    if let Err(r) = admin.require_same_tenant(Some(&req.data.x_tenant_id)) {
        return r;
    }
    let Ok(target) = req.data.user_id.trim().parse::<uuid::Uuid>() else {
        return not_found();
    };
    if target == admin.user_id {
        return sesame_common::tenant_admin::forbidden(
            "Use your own security page to change your sign-in methods",
        );
    }
    if let Err(r) = crate::controllers::user_status::may_change_status_of(&admin, target) {
        return r;
    }
    let tenant = admin.tenant.clone();
    let (t, u) = (tenant.clone(), target.to_string());
    let result = sesame_idam_database::with_pre_auth_tenant(&tenant, |exec| {
        let Some(user) = UserAdminService::find_by_id(&tenant, target, exec)? else {
            return Ok(None);
        };
        let removed = (exec as &dyn LifeExecutor).execute_values(
            "DELETE FROM sesame_idam.user_mfa_factors WHERE tenant_id = $1 AND user_id = $2::text::uuid",
            &sea_query::Values(vec![t.clone().into(), u.clone().into()]),
        )?;
        (exec as &dyn LifeExecutor).execute_values(
            "DELETE FROM sesame_idam.user_recovery_codes WHERE tenant_id = $1 AND user_id = $2::text::uuid",
            &sea_query::Values(vec![t.clone().into(), u.clone().into()]),
        )?;
        Ok(Some((user.email, removed)))
    });
    let (email, removed) = match result {
        Ok(Some(v)) => v,
        Ok(None) => return not_found(),
        Err(e) => {
            tracing::error!(error = %e, "disable_user_mfa: reset failed");
            return HttpJson::new(
                500,
                serde_json::json!({ "error": "internal_error", "error_description": "An unexpected error occurred" }),
            );
        }
    };
    match AuditLogEntry::new(
        AuditEventType::AuthenticatorChanged,
        "identity-user-mgmt-service",
    )
    .tenant_id(tenant.clone())
    .user_id(target.to_string())
    .decision_source("admin_mfa_reset")
    .result("allowed")
    .reason(format!(
        "reset by {} ({removed} factors removed)",
        admin.user_id
    ))
    .build()
    {
        Ok(entry) => EMITTER.emit(entry),
        Err(e) => tracing::warn!(error = %e, "disable_user_mfa: audit build failed"),
    }
    let body = format!(
        "An administrator of your organisation removed every second factor, passkey and recovery code from your \
         account ({removed} sign-in methods), at your request or as part of account recovery.\n\nWhen: {} UTC\n\n\
         Sign in with your password and set up a new authenticator app or passkey. If you did not ask for this, \
         contact your administrator now.\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M")
    );
    may::go!(move || {
        if let Err(e) = sesame_common::smtp::send_email(
            &email,
            "Security alert: your sign-in methods were reset",
            &body,
        ) {
            tracing::warn!(error = %format!("{e:#}"), "disable_user_mfa: notification not sent");
        }
    });
    HttpJson::ok(serde_json::json!({ "user_id": target.to_string(), "factors_removed": removed }))
}

fn not_found() -> HttpJson<serde_json::Value> {
    HttpJson::new(
        404,
        serde_json::json!({ "error": "not_found", "error_description": "Not found" }),
    )
}
