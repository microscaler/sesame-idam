//! Shared post-authentication issuance: once ANY factor has verified the
//! user (password, email OTP, magic link, …), the same enrichment + token
//! pipeline runs — authz-core role enrichment (graceful degrade), active-org
//! resolution, Ed25519 token issuance — and the same `TokenResponse`-shaped
//! JSON body is returned. Mirrors `auth_login`'s success path so every
//! factor issues identical tokens.

use crate::services::token_issuer;

/// Default portal/client for direct browser logins (matches `auth_login`).
const DEFAULT_PORTAL: &str = "frontend";

/// Build the full login-success response body for a verified user.
///
/// # Errors
///
/// Returns an error string suitable for logging when token issuance or
/// serialization fails — the caller maps it to the generic 500.
pub fn issue_login_response(
    user_id: &str,
    tenant_id: &str,
    preferred_org: Option<&str>,
) -> Result<serde_json::Value, String> {
    let authz = crate::services::authz_client::fetch_effective_authz(
        user_id,
        tenant_id,
        DEFAULT_PORTAL,
    )
    .unwrap_or_else(|e| {
        tracing::warn!(error = %e, "login_success: authz enrichment failed — issuing without roles");
        crate::services::authz_client::EffectiveAuthz {
            roles: vec![],
            permissions: vec![],
        }
    });

    let exec = sesame_idam_database::db();
    let active_org = crate::services::org_context::resolve_active_org_id(
        exec,
        user_id,
        tenant_id,
        preferred_org,
    );
    let org_id_str = active_org.map(|id| id.to_string());

    let tokens = token_issuer::issue_tokens(
        user_id,
        tenant_id,
        DEFAULT_PORTAL,
        authz.roles.clone(),
        authz.permissions,
        "customer",
        org_id_str.as_deref(),
    )
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
