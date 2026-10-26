// BRRTRouter: user-owned

//! `POST /auth/mfa/recovery-codes` — a new set of recovery codes, shown once; the old set stops working
//! (management session only, and only for an account with a second factor). The account holder is emailed.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::mfa_recovery_codes::Request;

use crate::services::mfa::factors;
use crate::services::mfa::notify::{self, Event};
use crate::services::mfa::{self, context::Mode};

#[handler(MfaRecoveryCodesController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let ctx = match mfa::context_for(req.data.mfa_token.trim(), &[Mode::Manage]) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    match factors::list_active(&ctx.tenant_id, ctx.user_id) {
        Ok(active) if active.is_empty() => {
            return mfa::error(
                409,
                "mfa_required",
                "Add an authenticator app or a passkey first; recovery codes stand in for one.",
            );
        }
        Ok(_) => {}
        Err(e) => {
            tracing::error!(error = %e, "mfa_recovery_codes: lookup failed");
            return mfa::unavailable();
        }
    }
    match mfa::regenerate_recovery_codes(&ctx.tenant_id, ctx.user_id) {
        Ok(codes) => {
            notify::announce(
                &ctx.tenant_id,
                &ctx.user_id,
                &ctx.email,
                Event::RecoveryCodesGenerated,
                "",
            );
            HttpJson::ok(serde_json::json!({ "recovery_codes": codes }))
        }
        Err(e) => {
            tracing::error!(error = %e, "mfa_recovery_codes: generation failed");
            mfa::unavailable()
        }
    }
}
