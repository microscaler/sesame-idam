// BRRTRouter: user-owned

//! `POST /auth/mfa/manage` — a signed-in user opens a sign-in security session (15 minutes) for the factor
//! management endpoints (list, add, rename, remove, recovery codes).
//!
//! Changing how an account signs in is the most valuable thing a stolen session can do, so the bearer token must be
//! from a recent sign-in (`auth_time` within [`FRESH_SECS`]: re-authenticate otherwise, `login_required`), and when
//! the account already has a second factor, from a sign-in that used one (`mfa_required`): an older or password-only
//! session can never remove a factor or add an attacker's.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_common::authn::Authn;
use sesame_idam_identity_login_service_gen::handlers::mfa_manage::Request;

use crate::auth_context::authenticated_principal;
use crate::services::mfa;
use crate::services::mfa::context::{self, MfaContext, Mode};
use crate::services::user_service::{UserService, STATUS_ACTIVE};

/// How recent the sign-in must be.
pub const FRESH_SECS: i64 = 600;

#[handler(MfaManageController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let (user_id, tenant_id) = match authenticated_principal(&req.jwt_claims, None) {
        Ok(p) => p,
        Err(e) => return e,
    };
    let claims = req.jwt_claims.clone().unwrap_or_default();
    let Some(authn) = Authn::from_claims(&claims) else {
        return login_required();
    };
    let now = chrono::Utc::now().timestamp();
    if now - authn.auth_time > FRESH_SECS {
        return login_required();
    }
    let user = match sesame_idam_database::with_pre_auth_tenant(&tenant_id, |exec| {
        UserService::find_by_tenant_and_id(&tenant_id, user_id, exec)
    }) {
        Ok(Some(u)) if u.status == STATUS_ACTIVE => u,
        Ok(_) => return login_required(),
        Err(e) => {
            tracing::error!(error = %e, "mfa_manage: user lookup failed");
            return mfa::unavailable();
        }
    };
    let (held, required) = match mfa::load_state(&tenant_id, user_id) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error = %e, "mfa_manage: factor lookup failed");
            return mfa::unavailable();
        }
    };
    if held.any_factor() && !authn.has("mfa") {
        return mfa::error(
            401,
            "mfa_required",
            "Sign in with your second factor to change your sign-in methods.",
        );
    }
    let portal = claims
        .get("client_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(crate::services::login_success::DEFAULT_PORTAL)
        .to_string();
    let ctx = MfaContext {
        mode: Mode::Manage,
        tenant_id,
        user_id,
        email: user.email,
        portal,
        organization_id: None,
        methods: vec![],
        auth_time: authn.auth_time,
        required,
        attempts: 0,
    };
    match context::create(&ctx) {
        Ok((token, ttl)) => HttpJson::ok(serde_json::json!({
            "mfa_token": token,
            "expires_in": ttl,
        })),
        Err(e) => {
            tracing::error!(error = %e, "mfa_manage: context store failed");
            mfa::unavailable()
        }
    }
}

fn login_required() -> HttpJson<serde_json::Value> {
    HttpJson::new(
        401,
        serde_json::json!({
            "error": "login_required",
            "error_description": "Sign in again to change your sign-in methods."
        }),
    )
}
