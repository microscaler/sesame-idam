// BRRTRouter: user-owned

//! `POST /auth/magic-link` — request an email magic link.
//!
//! Gate A3 + the email slice: abuse-guarded (shares the email channel budget
//! with email OTP so mixed endpoints cannot multiply a mailbox flood); when
//! allowed and the account exists+is active, a single-use 256-bit token is
//! minted (hashed in Redis, TTL'd) and the clickable URL is delivered via
//! SMTP. Generic success response regardless of suppression or provider
//! outcome.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::magic_link_send::Request;

use crate::services::abuse_guard::{self, Channel};
use crate::services::tenant_gate::tenant_http_error;
use crate::services::tenant_service::TenantService;
use crate::services::user_service::{UserService, STATUS_ACTIVE};
use crate::services::{email, otp};

#[handler(MagicLinkSendController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let tenant_id = req.data.x_tenant_id.clone();
    let recipient = req.data.email.clone();

    let exec = sesame_idam_database::db();
    if let Err(e) = TenantService::require_active(tenant_id.trim(), exec) {
        return tenant_http_error(&e);
    }

    let decision = abuse_guard::gate_otp_send(&tenant_id, Channel::Email, &recipient);
    if !decision.should_send() {
        tracing::info!(tenant = %tenant_id, ?decision, "magic link send suppressed by abuse guard");
        return generic_success();
    }

    let user = sesame_idam_database::with_pre_auth_tenant(&tenant_id, |exec| {
        UserService::find_by_tenant_and_email(&tenant_id, &recipient, exec)
    })
    .ok()
    .flatten();

    match user {
        Some(user) if user.status == STATUS_ACTIVE => {
            match otp::create_magic_link(&tenant_id, &user.id.to_string()) {
                Ok(url) => {
                    let body = format!(
                        "Sign in to Sesame with this link:\n\n{url}\n\nIt expires in 10 minutes and can be used once. If you did not request it, ignore this email."
                    );
                    if let Err(e) = email::send_email(&recipient, "Your sign-in link", &body) {
                        tracing::error!(error = %e, tenant = %tenant_id, "magic link delivery failed");
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, tenant = %tenant_id, "magic link mint failed");
                }
            }
        }
        _ => {
            tracing::info!(tenant = %tenant_id, "magic link requested for unknown/inactive account — no send");
        }
    }

    generic_success()
}

fn generic_success() -> HttpJson<serde_json::Value> {
    HttpJson::ok(serde_json::json!({
        "success": true,
        "message": "If the account exists, a link has been sent"
    }))
}
