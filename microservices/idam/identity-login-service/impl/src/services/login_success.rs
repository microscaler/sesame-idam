//! Shared post-authentication issuance: once a ceremony is complete (the MFA gate in [`crate::services::mfa`] said
//! so), the same enrichment + token pipeline runs — authz-core role enrichment (graceful degrade), active-org
//! resolution, Ed25519 token issuance with the ceremony's `amr`/`acr`/`auth_time` — and the same
//! `TokenResponse`-shaped JSON body is returned, whichever factor signed the user in.

use brrtrouter::typed::HttpJson;
use sesame_common::authn::{Authn, Method};
use uuid::Uuid;

use crate::services::mfa::{self, Subject};
use crate::services::token_issuer;
use crate::services::user_service::UserService;

/// Default portal/client for direct browser logins without a registered client (email code, magic link, SMS).
pub const DEFAULT_PORTAL: &str = "frontend";

/// Build the full login-success response body for a user whose ceremony is complete.
///
/// # Errors
///
/// Returns an error string suitable for logging when token issuance or serialization fails — the caller maps it to
/// the generic 500.
pub fn issue_session(
    user_id: &str,
    tenant_id: &str,
    portal: &str,
    preferred_org: Option<&str>,
    ceremony: &Authn,
) -> Result<serde_json::Value, String> {
    use sesame_common::observability::metrics::login_phase;
    // authz-core enrichment (an HTTP call) and the active-organisation lookup (a query) do not depend on each other:
    // the call runs in its own coroutine while this one resolves the organisation, so a sign-in waits for the slower
    // of the two, not their sum. Both run only after the ceremony is complete, as before.
    let (u, t, p) = (
        user_id.to_string(),
        tenant_id.to_string(),
        portal.to_string(),
    );
    let enrichment = may::go!(move || login_phase("authz", || {
        crate::services::authz_client::fetch_effective_authz(&u, &t, &p)
    }));
    let exec = sesame_idam_database::db();
    let active_org = login_phase("org", || {
        crate::services::org_context::resolve_active_org_id(exec, user_id, tenant_id, preferred_org)
    });
    let authz = enrichment
        .join()
        .unwrap_or_else(|_| Err("authz enrichment coroutine panicked".to_string()))
        .unwrap_or_else(|e| {
            sesame_common::observability::metrics::DEPENDENCY_ERRORS
                .with_label_values(&["authz-core", "login_enrichment"])
                .inc();
            tracing::warn!(error = %e, "login_success: authz enrichment failed — issuing without roles");
            crate::services::authz_client::EffectiveAuthz {
                roles: vec![],
                permissions: vec![],
            }
        });

    let org_id_str = active_org.map(|id| id.to_string());

    let tokens = login_phase("sign", || {
        token_issuer::issue_tokens(
            user_id,
            tenant_id,
            portal,
            authz.roles.clone(),
            authz.permissions,
            "customer",
            org_id_str.as_deref(),
            Some(ceremony),
        )
    })
    .map_err(|e| format!("token issuance failed: {e}"))?;

    Ok(serde_json::json!({
        "access_token": tokens.access_token,
        "expires_in": i32::try_from(tokens.expires_in).unwrap_or(300),
        "mfa_required": false,
        "refresh_token": tokens.refresh_token,
        "refresh_token_expires_in": i32::try_from(tokens.refresh_expires_in).unwrap_or(i32::MAX),
        "roles": authz.roles,
        "scope": tokens.scope,
        "token_type": "Bearer",
        "token_version": i32::try_from(tokens.token_version).ok(),
        "user_id": user_id,
    }))
}

/// A first factor without a registered client (email code, magic link, SMS) passed for `user_id`: run the MFA gate.
/// `200` with tokens, `202` with the next step, or an error body.
#[must_use]
pub fn first_factor_passed(
    user_id: &str,
    tenant_id: &str,
    preferred_org: Option<&str>,
    method: Method,
) -> HttpJson<serde_json::Value> {
    let Ok(uid) = Uuid::parse_str(user_id) else {
        return internal_error();
    };
    let user = match sesame_idam_database::with_pre_auth_tenant(tenant_id, |exec| {
        UserService::find_by_tenant_and_id(tenant_id, uid, exec)
    }) {
        Ok(Some(user)) => user,
        Ok(None) => return internal_error(),
        Err(e) => {
            tracing::error!(error = %e, "login_success: user lookup failed");
            return internal_error();
        }
    };
    mfa::after_first_factor(
        &Subject {
            tenant_id: tenant_id.to_string(),
            user_id: uid,
            email: user.email,
            portal: DEFAULT_PORTAL.to_string(),
            organization_id: preferred_org.map(str::to_string),
        },
        method,
    )
}

fn internal_error() -> HttpJson<serde_json::Value> {
    HttpJson::new(
        500,
        serde_json::json!({
            "error": "internal_error",
            "error_description": "An unexpected error occurred"
        }),
    )
}
