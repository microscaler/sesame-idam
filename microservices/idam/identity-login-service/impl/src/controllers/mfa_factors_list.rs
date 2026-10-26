// BRRTRouter: user-owned

//! `POST /auth/mfa/factors` — the account's sign-in methods, for the security page (management session only).

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::mfa_factors_list::Request;

use crate::services::mfa::{self, context::Mode};

#[handler(MfaFactorsListController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    match mfa::context_for(req.data.mfa_token.trim(), &[Mode::Manage]) {
        Ok(ctx) => mfa::factors_response(&ctx),
        Err(response) => response,
    }
}
