//! Access + refresh token issuance for login/register.
//!
//! Signs real Ed25519 (`EdDSA`, `typ=at+jwt`) access tokens with the shared
//! signing key (`SESAME_JWT_SIGNING_KEY_PKCS8_B64` / `SESAME_JWT_SIGNING_KID`
//! — the same key whose public half identity-session-service publishes in
//! JWKS), and seeds the refresh-token state in Redis so the session service's
//! `/auth/refresh` rotation can operate on tokens issued here.

use std::sync::LazyLock;

use sesame_common::jwt::{AccessClaimsBuilder, Ed25519Signer, SesameAuthzClaimsBuilder};
use sesame_common::token_versioning::VersionStore;
use uuid::Uuid;

use crate::jwt::ttl::TtlConfig;
use crate::models::refresh_token::{RefreshToken, REFRESH_TOKEN_TTL};

/// Process-wide signer. Loads the shared key from env; falls back to an
/// ephemeral dev key (with a warning) when unconfigured.
pub static SIGNER: LazyLock<Ed25519Signer> = LazyLock::new(|| {
    Ed25519Signer::from_env_or_generate()
        .expect("Failed to initialize JWT signer — invalid signing key material")
});

/// Issuer URL placed in the `iss` claim. Must be in
/// `sesame_common::jwt::ALLOWED_ISSUERS` for consumers to accept the token.
fn issuer() -> String {
    std::env::var("SESAME_JWT_ISSUER").unwrap_or_else(|_| "https://idam.example.com".to_string())
}

/// Tokens issued for a successful login/register.
pub struct IssuedTokens {
    pub access_token: String,
    pub refresh_token: String,
    /// Access token TTL in seconds.
    pub expires_in: i64,
    /// Refresh token TTL in seconds.
    pub refresh_expires_in: i64,
    /// Session id embedded in the claims.
    pub session_id: String,
    /// Token version (`ver` claim).
    pub token_version: u64,
    pub scope: String,
}

/// Errors from token issuance.
#[derive(Debug, thiserror::Error)]
pub enum IssueError {
    #[error("claims construction failed: {0}")]
    Claims(String),
    #[error("signing failed: {0}")]
    Signing(String),
}

/// Issue an access + refresh token pair for a user.
///
/// - `ver` comes from the Redis version store when available, else 1.
/// - The refresh token is a signed JWT carrying `jti`/`sid`/`family_id`,
///   with matching metadata stored under `refresh:{jti}` in Redis.
///
/// # Errors
///
/// Returns [`IssueError`] if claims fail validation or signing fails.
/// Redis unavailability degrades gracefully (logged): the access token is
/// still issued, but `/auth/refresh` will reject the refresh token until
/// Redis is back.
pub fn issue_tokens(
    user_id: &str,
    tenant_id: &str,
    portal: &str,
    roles: Vec<String>,
    permissions: Vec<String>,
    role_for_ttl: &str,
    org_id: Option<&str>,
) -> Result<IssuedTokens, IssueError> {
    let ttl_config = TtlConfig::from_env();
    let now = chrono::Utc::now().timestamp();
    let access_ttl = i64::try_from(ttl_config.ttl_for_role(role_for_ttl).as_secs()).unwrap_or(300);
    let scope = "openid profile email".to_string();

    let session_id = Uuid::new_v4().to_string();
    let access_jti = Uuid::new_v4().to_string();

    // Token version from the Redis version store (Story 5); fall back to 1.
    let token_version = match VersionStore::from_url(
        &std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into()),
    ) {
        Ok(store) => match store.issue_version(user_id) {
            Ok((ver, _ttl)) => ver,
            Err(e) => {
                tracing::warn!(error = %e, "version store unavailable — using ver=1");
                1
            }
        },
        Err(e) => {
            tracing::warn!(error = %e, "version store init failed — using ver=1");
            1
        }
    };

    let sx = SesameAuthzClaimsBuilder::new()
        .tenant(tenant_id)
        .portal(portal)
        .roles(roles)
        .permissions(permissions)
        .build()
        .map_err(|e| IssueError::Claims(e.to_string()))?;

    let claims = AccessClaimsBuilder::new()
        .iss(issuer())
        .sub(user_id)
        .aud(vec!["sesame-idam".to_string()])
        .client_id(portal)
        .scope(scope.clone())
        .exp(now + access_ttl)
        .nbf(now)
        .iat(now)
        .jti(access_jti)
        .ver(token_version)
        .sid(session_id.clone())
        .tenant_id(tenant_id)
        .user_id(user_id)
        .user_type("customer")
        .org_id_opt(org_id.map(str::to_string))
        .sx(sx)
        .build()
        .map_err(|e| IssueError::Claims(e.to_string()))?;

    let access_token = SIGNER
        .sign_access_claims(&claims)
        .map_err(|e| IssueError::Signing(e.to_string()))?;

    // ── Refresh token ────────────────────────────────────────────────────
    let refresh_jti = Uuid::new_v4().to_string();
    let refresh_exp = now + i64::from(REFRESH_TOKEN_TTL);
    // Family id = session id: all rotations of this session share a family.
    let family_id = session_id.clone();

    let refresh_payload = serde_json::json!({
        "jti": refresh_jti,
        "sub": user_id,
        "sid": session_id,
        "family_id": family_id,
        "iat": now,
        "exp": refresh_exp,
        "typ": "refresh",
    });
    let refresh_token = SIGNER
        .sign_payload(&refresh_payload.to_string())
        .map_err(|e| IssueError::Signing(e.to_string()))?;

    let metadata = RefreshToken {
        jti: refresh_jti,
        sub: user_id.to_string(),
        sid: session_id.clone(),
        family_id,
        iat: now,
        exp: refresh_exp,
        client_id: portal.to_string(),
        scopes: scope.clone(),
    };
    if let Err(e) = crate::redis::store_refresh_token(&metadata) {
        tracing::warn!(
            error = %e,
            "failed to store refresh token in Redis — /auth/refresh will reject it"
        );
    }

    Ok(IssuedTokens {
        access_token,
        refresh_token,
        expires_in: access_ttl,
        refresh_expires_in: i64::from(REFRESH_TOKEN_TTL),
        session_id,
        token_version,
        scope,
    })
}
