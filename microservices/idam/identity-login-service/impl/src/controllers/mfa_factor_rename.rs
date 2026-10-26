// BRRTRouter: user-owned

//! `POST /auth/mfa/factors/rename` — rename an authenticator app or passkey (management session only).

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::mfa_factor_rename::Request;

use crate::services::mfa::{self, context::Mode, factors};

#[handler(MfaFactorRenameController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let ctx = match mfa::context_for(req.data.mfa_token.trim(), &[Mode::Manage]) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    let Ok(factor_id) = uuid::Uuid::parse_str(req.data.factor_id.trim()) else {
        return mfa::error(404, "not_found", "No such sign-in method.");
    };
    let label = req.data.label.trim();
    if label.is_empty() || label.chars().count() > 64 {
        return mfa::error(
            400,
            "invalid_request",
            "A name of 1 to 64 characters is required.",
        );
    }
    match factors::rename(&ctx.tenant_id, ctx.user_id, factor_id, label) {
        Ok(true) => mfa::factors_response(&ctx),
        Ok(false) => mfa::error(404, "not_found", "No such sign-in method."),
        Err(e) => {
            tracing::error!(error = %e, "mfa_factor_rename: update failed");
            mfa::unavailable()
        }
    }
}
