// BRRTRouter: user-owned

//! Pre-registration eligibility check (`GET /auth/signup/validate`).
//!
//! Tenant-scoped, read-only: reports whether an email is available to register.
//! Never creates state. Consumed by the Hauliage BFF before showing the signup
//! form. `POST /auth/register` remains the authoritative gate (and the DB
//! `UNIQUE(tenant_id, email)` constraint the failsafe); this is a UX pre-check.

use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::signup_validate::{Request, Response};

use crate::services::tenant_service::{TenantGateError, TenantService};
use crate::services::user_service::UserService;

#[handler(SignupValidateController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    let tenant_id = req.data.x_tenant_id.trim();
    let email = req.data.email.as_deref().map(str::trim).unwrap_or_default();

    let mut reasons: Vec<String> = Vec::new();

    if email.is_empty() {
        reasons.push("email_required".to_string());
    } else if !is_plausible_email(email) {
        reasons.push("email_invalid".to_string());
    } else if tenant_id.is_empty() {
        reasons.push("tenant_required".to_string());
    } else {
        let exec = sesame_idam_database::db();
        match TenantService::require_active(tenant_id, exec) {
            Err(TenantGateError::Unknown) => reasons.push("tenant_unknown".to_string()),
            Err(TenantGateError::NotActive) => reasons.push("tenant_not_active".to_string()),
            Err(TenantGateError::Db(_)) => reasons.push("validation_unavailable".to_string()),
            Ok(_) => {
                match sesame_idam_database::with_pre_auth_tenant(tenant_id, |exec| {
                    UserService::find_by_tenant_and_email(tenant_id, &email.to_lowercase(), exec)
                }) {
                    Ok(Some(_)) => reasons.push("email_taken".to_string()),
                    Ok(None) => {}
                    Err(e) => {
                        tracing::error!(error = %e, "signup_validate: availability check failed");
                        reasons.push("validation_unavailable".to_string());
                    }
                }
            }
        }
    }

    Response {
        allowed: reasons.is_empty(),
        reasons: Some(reasons),
        requires_mfa: Some(false),
    }
}

/// Minimal plausibility check — exactly one `@`, non-empty local part, and a
/// dotted domain. Authoritative RFC 5322 validation happens at register time.
fn is_plausible_email(email: &str) -> bool {
    let mut parts = email.split('@');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(local), Some(domain), None) => {
            !local.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
        }
        _ => false,
    }
}
