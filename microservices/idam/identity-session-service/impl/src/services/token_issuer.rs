//! Access + refresh token issuance during `/session/refresh` rotation.
//!
//! Uses the same Ed25519 signing key as identity-login-service
//! (`SESAME_JWT_SIGNING_KEY_PKCS8_B64` / `SESAME_JWT_SIGNING_KID`) so rotated
//! access tokens validate against the JWKS endpoint.

use std::sync::LazyLock;

use sesame_common::jwt::{AccessClaimsBuilder, Ed25519Signer, SesameAuthzClaimsBuilder};
use uuid::Uuid;

use crate::models::refresh_token::{RefreshToken, REFRESH_TOKEN_TTL};

/// Default access-token TTL (seconds) when env/config is unavailable.
const DEFAULT_ACCESS_TTL_SECS: i64 = 300;

/// Process-wide signer — same key-source configuration as login-service
/// (shared keyset file when `KEY_SOURCE=file`, else env pair, else a development-only key).
pub static SIGNER: LazyLock<Ed25519Signer> = LazyLock::new(|| {
    Ed25519Signer::from_configured()
        .expect("Failed to initialize JWT signer — invalid signing key material")
});

fn issuer() -> String {
    sesame_common::deployment::issuer()
}

fn access_ttl_secs() -> i64 {
    std::env::var("JWT_ACCESS_TTL_NORMAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_ACCESS_TTL_SECS)
}

/// Tokens issued after a successful refresh rotation.
pub struct IssuedRefreshPair {
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_in: i32,
    pub refresh_expires_in: i32,
    pub user_id: String,
    pub scope: String,
}

/// Errors from refresh-time token issuance.
#[derive(Debug, thiserror::Error)]
pub enum IssueError {
    #[error("claims construction failed: {0}")]
    Claims(String),
    #[error("signing failed: {0}")]
    Signing(String),
}

/// Issue a new access + refresh pair for a rotated session.
///
/// Reuses the session id and family from the stored refresh metadata. The
/// `ver` claim reflects the current Redis version (not bumped on refresh).
///
/// # Errors
///
/// Returns [`IssueError`] when claim validation or signing fails.
pub fn issue_rotated_tokens(
    token: &RefreshToken,
    tenant_id: &str,
    token_version: u64,
    ceremony: Option<&sesame_common::authn::Authn>,
) -> Result<IssuedRefreshPair, IssueError> {
    let now = chrono::Utc::now().timestamp();
    let access_ttl = access_ttl_secs();
    let scope = token.scopes.clone();
    let access_jti = Uuid::new_v4().to_string();

    // `token_version`: the subject's version, read (and initialised) by the rotation's first Redis round trip with
    // exactly VersionStore::issue_version's semantics (SET NX 1, GET, EXPIRE 360 s); it used to open a connection here.
    // Re-fetch effective roles + permissions from authz-core so a refresh
    // reflects the user's CURRENT authorization: a role granted after the
    // session was established takes effect on the next refresh, not only on a
    // fresh sign-in. This previously minted `roles: vec![]`, so a user
    // silently lost every role ~one access-TTL after login (the first
    // rotation). Degrades to empty on failure — the same graceful-degradation
    // contract as login-time enrichment; the client refreshes again once
    // authz-core is back.
    let authz = crate::services::authz_client::fetch_effective_authz(
        &token.sub,
        tenant_id,
        &token.client_id,
    )
    .unwrap_or_else(|e| {
        tracing::warn!(
            error = %e,
            "issue_rotated_tokens: authz-core enrichment failed — refreshing token without roles"
        );
        crate::services::authz_client::EffectiveAuthz {
            roles: vec![],
            permissions: vec![],
        }
    });

    let sx = SesameAuthzClaimsBuilder::new()
        .tenant(tenant_id)
        .portal(&token.client_id)
        .roles(authz.roles)
        .permissions(authz.permissions)
        .build()
        .map_err(|e| IssueError::Claims(e.to_string()))?;

    let claims = AccessClaimsBuilder::new()
        .iss(issuer())
        .sub(&token.sub)
        .aud(sesame_common::deployment::issue_audiences())
        .client_id(&token.client_id)
        .scope(scope.clone())
        .exp(now + access_ttl)
        .nbf(now)
        .iat(now)
        .jti(access_jti)
        .ver(token_version)
        .sid(&token.sid)
        .tenant_id(tenant_id)
        .user_id(&token.sub)
        .user_type("customer")
        .sx(sx)
        .authn(ceremony.cloned())
        .build()
        .map_err(|e| IssueError::Claims(e.to_string()))?;

    let access_token = SIGNER
        .sign_access_claims(&claims)
        .map_err(|e| IssueError::Signing(e.to_string()))?;

    // The refresh JWT MUST carry the jti the rotated token was STORED under
    // (rotate_refresh_token_bound minted `token.jti` and put it in Redis).
    // This used to mint a second, unrelated UUID, so the token handed to
    // the client was never in Redis: the second refresh of every session
    // was `not_found` -> invalid_grant, and every client signed out about
    // two access-token lifetimes after login. The "random logout" in
    // hauliage and PriceWhisperer was this line.
    let refresh_jti = token.jti.clone();
    let refresh_exp = token.exp;
    let mut refresh_payload = serde_json::json!({
        "jti": refresh_jti,
        "sub": token.sub,
        "sid": token.sid,
        "family_id": token.family_id,
        "iat": now,
        "exp": refresh_exp,
        "typ": "refresh",
    });
    // The sign-in's authentication context rides along unchanged (ADR-008 §3: auth_time is when the user
    // authenticated, not when the token was refreshed).
    if let Some(ceremony) = ceremony {
        ceremony.write_into(&mut refresh_payload);
    }
    let refresh_token = SIGNER
        .sign_payload(&refresh_payload.to_string())
        .map_err(|e| IssueError::Signing(e.to_string()))?;

    Ok(IssuedRefreshPair {
        access_token,
        refresh_token,
        access_expires_in: i32::try_from(access_ttl).unwrap_or(i32::MAX),
        // The successor's own remaining lifetime: the idle lifetime, or less near the session's absolute expiry.
        refresh_expires_in: i32::try_from((token.exp - now).clamp(0, i64::from(REFRESH_TOKEN_TTL)))
            .unwrap_or(i32::MAX),
        user_id: token.sub.clone(),
        scope,
    })
}
