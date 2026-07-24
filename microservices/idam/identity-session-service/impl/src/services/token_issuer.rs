//! Access + refresh token issuance during `/session/refresh` rotation.
//!
//! Uses the same Ed25519 signing key as identity-login-service
//! (`SESAME_JWT_SIGNING_KEY_PKCS8_B64` / `SESAME_JWT_SIGNING_KID`) so rotated
//! access tokens validate against the JWKS endpoint.

use std::sync::LazyLock;

use sesame_common::jwt::{AccessClaimsBuilder, Ed25519Signer, SesameAuthzClaimsBuilder};
use sesame_common::token_versioning::VersionStore;
use uuid::Uuid;

use crate::models::refresh_token::{RefreshToken, REFRESH_TOKEN_TTL};

/// Default access-token TTL (seconds) when env/config is unavailable.
const DEFAULT_ACCESS_TTL_SECS: i64 = 300;

/// Process-wide signer — same key-source configuration as login-service
/// (shared keyset file when `KEY_SOURCE=file`, else env pair, else dev key).
pub static SIGNER: LazyLock<Ed25519Signer> = LazyLock::new(|| {
    Ed25519Signer::from_configured()
        .expect("Failed to initialize JWT signer — invalid signing key material")
});

fn issuer() -> String {
    std::env::var("SESAME_JWT_ISSUER").unwrap_or_else(|_| "https://idam.example.com".to_string())
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
) -> Result<IssuedRefreshPair, IssueError> {
    let now = chrono::Utc::now().timestamp();
    let access_ttl = access_ttl_secs();
    let scope = token.scopes.clone();
    let access_jti = Uuid::new_v4().to_string();

    let token_version = match VersionStore::from_url(
        &std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into()),
    ) {
        Ok(store) => store.issue_version(&token.sub).map_or_else(
            |error| {
                tracing::warn!(%error, "version store unavailable on refresh — using ver=1");
                1
            },
            |(version, _ttl)| version,
        ),
        Err(e) => {
            tracing::warn!(error = %e, "version store unavailable on refresh — using ver=1");
            1
        }
    };

    let sx = SesameAuthzClaimsBuilder::new()
        .tenant(tenant_id)
        .portal(&token.client_id)
        .roles(vec![])
        .build()
        .map_err(|e| IssueError::Claims(e.to_string()))?;

    let claims = AccessClaimsBuilder::new()
        .iss(issuer())
        .sub(&token.sub)
        .aud(vec!["sesame-idam".to_string()])
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
        .build()
        .map_err(|e| IssueError::Claims(e.to_string()))?;

    let access_token = SIGNER
        .sign_access_claims(&claims)
        .map_err(|e| IssueError::Signing(e.to_string()))?;

    let refresh_jti = Uuid::new_v4().to_string();
    let refresh_exp = now + i64::from(REFRESH_TOKEN_TTL);
    let refresh_payload = serde_json::json!({
        "jti": refresh_jti,
        "sub": token.sub,
        "sid": token.sid,
        "family_id": token.family_id,
        "iat": now,
        "exp": refresh_exp,
        "typ": "refresh",
    });
    let refresh_token = SIGNER
        .sign_payload(&refresh_payload.to_string())
        .map_err(|e| IssueError::Signing(e.to_string()))?;

    Ok(IssuedRefreshPair {
        access_token,
        refresh_token,
        access_expires_in: i32::try_from(access_ttl).unwrap_or(i32::MAX),
        refresh_expires_in: i32::try_from(REFRESH_TOKEN_TTL).unwrap_or(i32::MAX),
        user_id: token.sub.clone(),
        scope,
    })
}
