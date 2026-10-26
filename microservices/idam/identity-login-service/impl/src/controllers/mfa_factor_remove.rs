// BRRTRouter: user-owned

//! `POST /auth/mfa/factors/remove` — remove an authenticator app or passkey (management session only).
//!
//! Refused when the account's organisation policy needs what would be left without it: the last factor under an MFA
//! policy, the last passkey under a passkey policy (`409 policy_requires_factor`). Removing the last factor also
//! removes the recovery codes (there is nothing left for them to recover). The account holder is emailed.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::mfa_factor_remove::Request;

use crate::services::mfa::factors::{self, Kind};
use crate::services::mfa::notify::{self, Event};
use crate::services::mfa::policy::{self, Requirement};
use crate::services::mfa::{self, context::Mode};

#[handler(MfaFactorRemoveController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let ctx = match mfa::context_for(req.data.mfa_token.trim(), &[Mode::Manage]) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    let Ok(factor_id) = uuid::Uuid::parse_str(req.data.factor_id.trim()) else {
        return mfa::error(404, "not_found", "No such sign-in method.");
    };
    let (all, required) = match factors::list(&ctx.tenant_id, ctx.user_id)
        .and_then(|all| Ok((all, policy::required_for(&ctx.tenant_id, ctx.user_id)?)))
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error = %e, "mfa_factor_remove: lookup failed");
            return mfa::unavailable();
        }
    };
    let Some(target) = all.iter().find(|f| f.id == factor_id) else {
        return mfa::error(404, "not_found", "No such sign-in method.");
    };
    let remaining = all
        .iter()
        .filter(|f| f.active && f.id != factor_id)
        .collect::<Vec<_>>();
    if target.active {
        let still_ok = match required {
            Requirement::None => true,
            Requirement::Mfa => !remaining.is_empty(),
            Requirement::Passkey => remaining.iter().any(|f| f.kind == Kind::Passkey),
        };
        if !still_ok {
            return mfa::error(
                409,
                "policy_requires_factor",
                "Your organisation requires this sign-in method. Add another before removing it.",
            );
        }
    }
    match factors::remove(&ctx.tenant_id, ctx.user_id, factor_id) {
        Ok(true) => {}
        Ok(false) => return mfa::error(404, "not_found", "No such sign-in method."),
        Err(e) => {
            tracing::error!(error = %e, "mfa_factor_remove: delete failed");
            return mfa::unavailable();
        }
    }
    if remaining.is_empty() {
        if let Err(e) = factors::replace_recovery_codes(&ctx.tenant_id, ctx.user_id, &[]) {
            tracing::warn!(error = %e, "mfa_factor_remove: recovery codes not cleared");
        }
    }
    if target.active {
        let what = format!("{} \"{}\"", target.kind.as_str(), target.label);
        notify::announce(
            &ctx.tenant_id,
            &ctx.user_id,
            &ctx.email,
            Event::FactorRemoved,
            &what,
        );
    }
    mfa::factors_response(&ctx)
}
