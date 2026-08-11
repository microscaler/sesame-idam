// BRRTRouter: user-owned

//! Pre-registration eligibility check (`GET /auth/signup/validate`).
//!
//! Tenant is bound via registered `client_id` (public north–south). Optional
//! legacy `X-Tenant-ID` must match the client tenant when present.
//!
//! Tenant-scoped, read-only: reports whether an email is available to register.
//! Never creates state. Consumed by product BFFs before showing the signup
//! form. `POST /auth/register` remains the authoritative gate (and the DB
//! `UNIQUE(tenant_id, email)` constraint the failsafe); this is a UX pre-check.

use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::signup_validate::{Request, Response};

use crate::services::client_registry::{ClientRegistry, ClientRegistryError};
use crate::services::tenant_service::{TenantGateError, TenantService};
use crate::services::user_service::UserService;

#[handler(SignupValidateController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    let email = req.data.email.as_deref().map(str::trim).unwrap_or_default();
    let mut reasons: Vec<String> = Vec::new();

    if email.is_empty() {
        reasons.push("email_required".to_string());
        return Response {
            allowed: false,
            reasons: Some(reasons),
            requires_mfa: Some(false),
        };
    }
    if !is_plausible_email(email) {
        reasons.push("email_invalid".to_string());
        return Response {
            allowed: false,
            reasons: Some(reasons),
            requires_mfa: Some(false),
        };
    }

    let exec = sesame_idam_database::db();
    let binding = match ClientRegistry::resolve_active(&req.data.client_id, exec) {
        Ok(binding) => binding,
        Err(ClientRegistryError::Unknown | ClientRegistryError::NotActive) => {
            reasons.push("client_invalid".to_string());
            return Response {
                allowed: false,
                reasons: Some(reasons),
                requires_mfa: Some(false),
            };
        }
        Err(ClientRegistryError::InvalidPolicy(error)) => {
            tracing::error!(
                %error,
                client_id = %req.data.client_id,
                "signup_validate: invalid registered client policy"
            );
            reasons.push("validation_unavailable".to_string());
            return Response {
                allowed: false,
                reasons: Some(reasons),
                requires_mfa: Some(false),
            };
        }
        Err(ClientRegistryError::Db(error)) => {
            tracing::error!(%error, "signup_validate: client registry lookup failed");
            reasons.push("validation_unavailable".to_string());
            return Response {
                allowed: false,
                reasons: Some(reasons),
                requires_mfa: Some(false),
            };
        }
    };

    if req
        .data
        .x_tenant_id
        .as_deref()
        .is_some_and(|tenant| tenant.trim() != binding.tenant_id)
    {
        reasons.push("client_invalid".to_string());
        return Response {
            allowed: false,
            reasons: Some(reasons),
            requires_mfa: Some(false),
        };
    }

    let tenant_id = binding.tenant_id.as_str();
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
