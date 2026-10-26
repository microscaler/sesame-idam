//! OIDC Authorization Code + PKCE session and code lifecycle.

use anyhow::{Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use redis::Commands;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::services::client_registry::ClientBinding;

/// How long a pending authorization request (the hosted login page's `request_id`) lives: 30 minutes, so a sign-in
/// page left open while the user fetches a password or answers a call still works. It stays single-use (taken with
/// GETDEL on completion); the code it turns into is the short-lived, single-use secret.
pub const AUTHORIZATION_SESSION_TTL_SECS: u64 = 1800;
const AUTHORIZATION_CODE_TTL_SECS: u64 = 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorizationError {
    InvalidRequest(&'static str),
    InvalidClient,
    InvalidRedirectUri,
    UnsupportedResponseType,
    InvalidScope,
    ServerUnavailable,
}

impl AuthorizationError {
    #[must_use]
    pub fn oauth_code(&self) -> &'static str {
        match self {
            Self::InvalidRequest(_) | Self::InvalidRedirectUri => "invalid_request",
            Self::InvalidClient => "invalid_client",
            Self::UnsupportedResponseType => "unsupported_response_type",
            Self::InvalidScope => "invalid_scope",
            Self::ServerUnavailable => "temporarily_unavailable",
        }
    }

    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::InvalidRequest(description) => description,
            Self::InvalidClient => "The client is unknown, disabled, or unavailable",
            Self::InvalidRedirectUri => "The redirect URI is not registered for this client",
            Self::UnsupportedResponseType => "Only response_type=code is supported",
            Self::InvalidScope => "The requested scope is not registered for this client",
            Self::ServerUnavailable => "The authorization service is temporarily unavailable",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthorizationSession {
    pub client_id: String,
    pub tenant_id: String,
    pub application_id: String,
    pub redirect_uri: String,
    pub state: String,
    pub nonce: String,
    pub scopes: Vec<String>,
    pub code_challenge: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthorizationCode {
    pub client_id: String,
    pub tenant_id: String,
    pub application_id: String,
    pub redirect_uri: String,
    pub user_id: String,
    pub scopes: Vec<String>,
    pub nonce: String,
    pub code_challenge: String,
    pub auth_time: i64,
    /// The hosted sign-in's authentication context (`amr`, `acr`, `auth_time`; ADR-008 §3), handed to the tokens the
    /// code is redeemed for. Absent in codes minted before it existed (they live 60 s).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authn: Option<sesame_common::authn::Authn>,
    pub created_at: i64,
}

pub fn validate_authorization_request(
    binding: &ClientBinding,
    response_type: &str,
    redirect_uri: &str,
    state: &str,
    nonce: &str,
    scope: &str,
    code_challenge: &str,
    code_challenge_method: &str,
) -> Result<AuthorizationSession, AuthorizationError> {
    if response_type != "code"
        || !binding
            .policy
            .response_types
            .iter()
            .any(|value| value == "code")
    {
        return Err(AuthorizationError::UnsupportedResponseType);
    }
    if !binding.redirect_uris.iter().any(|registered| {
        sesame_common::oidc_client::redirect_uri_matches(registered, redirect_uri)
    }) {
        return Err(AuthorizationError::InvalidRedirectUri);
    }
    if !is_unpredictable_parameter(state) {
        return Err(AuthorizationError::InvalidRequest(
            "state must contain between 16 and 512 characters",
        ));
    }
    if !is_unpredictable_parameter(nonce) {
        return Err(AuthorizationError::InvalidRequest(
            "nonce must contain between 16 and 512 characters",
        ));
    }
    if code_challenge_method != "S256" || !is_pkce_value(code_challenge) {
        return Err(AuthorizationError::InvalidRequest(
            "PKCE code_challenge_method=S256 and a valid challenge are required",
        ));
    }

    let mut scopes = scope
        .split_ascii_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    scopes.sort();
    scopes.dedup();
    if !scopes.iter().any(|requested| requested == "openid")
        || scopes
            .iter()
            .any(|requested| !binding.policy.scopes.contains(requested))
    {
        return Err(AuthorizationError::InvalidScope);
    }

    Ok(AuthorizationSession {
        client_id: binding.client_id.clone(),
        tenant_id: binding.tenant_id.clone(),
        application_id: binding.application_id.clone(),
        redirect_uri: sesame_common::oidc_client::normalize_redirect_uri(redirect_uri)
            .map_err(|_| AuthorizationError::InvalidRedirectUri)?,
        state: state.to_string(),
        nonce: nonce.to_string(),
        scopes,
        code_challenge: code_challenge.to_string(),
        created_at: chrono::Utc::now().timestamp(),
    })
}

fn is_unpredictable_parameter(value: &str) -> bool {
    (16..=512).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~'))
}

fn is_pkce_value(value: &str) -> bool {
    (43..=128).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~'))
}

#[must_use]
pub fn verify_pkce_s256(code_verifier: &str, expected_challenge: &str) -> bool {
    if !is_pkce_value(code_verifier) {
        return false;
    }
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(code_verifier.as_bytes()));
    challenge == expected_challenge
}

/// A Redis connection from the process-wide pool (`sesame_common::redis_pool`): reused, not opened per call.
fn redis_connection() -> Result<sesame_common::redis_pool::PooledConnection<'static>> {
    sesame_common::redis_pool::pooled_connection().context("connect to Redis")
}

fn opaque_value() -> String {
    let mut bytes = [0_u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn hashed_key(prefix: &str, value: &str) -> String {
    format!("{prefix}:{}", hex::encode(Sha256::digest(value.as_bytes())))
}

/// Why an authorization session or code could not be used.
#[derive(Debug)]
pub enum LookupError {
    /// Never stored, expired, or already used: both are one-time values, deleted on first use.
    Absent,
    /// The code exists but is bound to another client, redirect URI or PKCE verifier. It is burned.
    BindingMismatch,
    /// The store could not be reached or answered with an error: the value may well still be valid.
    Unavailable(anyhow::Error),
}

impl LookupError {
    /// Short reason for spans and logs.
    #[must_use]
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Absent => "absent_expired_or_used",
            Self::BindingMismatch => "binding_mismatch",
            Self::Unavailable(_) => "store_unavailable",
        }
    }
}

/// One Redis command as a span under the request: the operation, which kind of key (never the key, which is a
/// bearer secret's hash) and how it ended (`stored`, `hit`, `miss` or `error`).
fn redis_span(operation: &'static str, key_kind: &'static str) -> tracing::Span {
    tracing::info_span!(
        "redis.command",
        otel.name = %format!("redis {operation}"),
        "db.system.name" = "redis",
        "db.operation.name" = operation,
        "sesame.key_kind" = key_kind,
        "sesame.outcome" = tracing::field::Empty,
        "error.type" = tracing::field::Empty,
        otel.status_description = tracing::field::Empty,
    )
}

fn redis_set_ex(key_kind: &'static str, key: &str, payload: String, ttl_secs: u64) -> Result<()> {
    let span = redis_span("SETEX", key_kind);
    // Not `in_scope` (may_tracing ADR-0001): the Redis call yields, and the coroutine may resume elsewhere.
    {
        let result = redis_connection().and_then(|mut connection| {
            connection
                .set_ex::<_, _, ()>(key, payload, ttl_secs)
                .with_context(|| format!("store {key_kind}"))
        });
        match &result {
            Ok(()) => {
                span.record("sesame.outcome", "stored");
            }
            Err(error) => {
                span.record("sesame.outcome", "error");
                span.record("error.type", "redis");
                span.record("otel.status_description", error.to_string().as_str());
                tracing::error!(key_kind, error = %error, "Redis SETEX failed");
            }
        }
        result
    }
}

/// GETDEL: the value, once. `Ok(None)` when absent (never stored, expired or already taken).
fn redis_getdel(key_kind: &'static str, key: &str) -> Result<Option<String>> {
    let span = redis_span("GETDEL", key_kind);
    // Not `in_scope` (may_tracing ADR-0001): the Redis call yields, and the coroutine may resume elsewhere.
    {
        let result = redis_connection().and_then(|mut connection| {
            redis::cmd("GETDEL")
                .arg(key)
                .query::<Option<String>>(&mut connection)
                .with_context(|| format!("take {key_kind}"))
        });
        match &result {
            Ok(value) => {
                span.record(
                    "sesame.outcome",
                    if value.is_some() { "hit" } else { "miss" },
                );
            }
            Err(error) => {
                span.record("sesame.outcome", "error");
                span.record("error.type", "redis");
                span.record("otel.status_description", error.to_string().as_str());
                tracing::error!(key_kind, error = %error, "Redis GETDEL failed");
            }
        }
        result
    }
}

pub fn store_authorization_session(session: &AuthorizationSession) -> Result<String> {
    let request_id = opaque_value();
    let payload = serde_json::to_string(session).context("serialize authorization session")?;
    redis_set_ex(
        "authorization_session",
        &hashed_key("oidc:authorization-request", &request_id),
        payload,
        AUTHORIZATION_SESSION_TTL_SECS,
    )?;
    Ok(request_id)
}

/// Take (and delete) the authorization session for `request_id`.
///
/// # Errors
/// [`LookupError::Absent`] when it expired ([`AUTHORIZATION_SESSION_TTL_SECS`]) or was already used;
/// [`LookupError::Unavailable`] when the store failed.
pub fn take_authorization_session(request_id: &str) -> Result<AuthorizationSession, LookupError> {
    let payload = redis_getdel(
        "authorization_session",
        &hashed_key("oidc:authorization-request", request_id),
    )
    .map_err(LookupError::Unavailable)?
    .ok_or(LookupError::Absent)?;
    serde_json::from_str(&payload).map_err(|error| {
        LookupError::Unavailable(anyhow::anyhow!("corrupt authorization session: {error}"))
    })
}

/// Seconds left on a pending authorization request, without using it: `Ok(None)` when it is absent (never stored,
/// expired or already used). Read-only (Redis `TTL`), so the hosted page can ask as often as it likes.
///
/// # Errors
/// The store could not be reached.
pub fn authorization_session_remaining(request_id: &str) -> Result<Option<u64>> {
    let span = redis_span("TTL", "authorization_session");
    let key = hashed_key("oidc:authorization-request", request_id);
    let result = redis_connection().and_then(|mut connection| {
        connection
            .ttl::<_, i64>(&key)
            .context("read authorization session TTL")
    });
    match result {
        Ok(reply) => {
            let remaining = remaining_from_ttl_reply(reply);
            span.record(
                "sesame.outcome",
                if remaining.is_some() { "hit" } else { "miss" },
            );
            Ok(remaining)
        }
        Err(error) => {
            span.record("sesame.outcome", "error");
            span.record("error.type", "redis");
            span.record("otel.status_description", error.to_string().as_str());
            tracing::error!(error = %error, "Redis TTL failed");
            Err(error)
        }
    }
}

/// Redis `TTL` reply to seconds left: `-2` is a missing key, `-1` a key without expiry (never written so; treated as
/// a full lifetime), anything else the seconds left.
#[must_use]
pub fn remaining_from_ttl_reply(reply: i64) -> Option<u64> {
    match reply {
        -1 => Some(AUTHORIZATION_SESSION_TTL_SECS),
        n if n > 0 => u64::try_from(n).ok(),
        _ => None,
    }
}

/// [`take_authorization_session`] without the reason.
pub fn consume_authorization_session(request_id: &str) -> Option<AuthorizationSession> {
    take_authorization_session(request_id).ok()
}

pub fn mint_authorization_code(code: &AuthorizationCode) -> Result<String> {
    let value = opaque_value();
    let payload = serde_json::to_string(code).context("serialize authorization code")?;
    redis_set_ex(
        "authorization_code",
        &hashed_key("oidc:authorization-code", &value),
        payload,
        AUTHORIZATION_CODE_TTL_SECS,
    )?;
    Ok(value)
}

/// Redeem (and burn) an authorization code for the client, redirect URI and PKCE verifier it was issued to.
///
/// # Errors
/// [`LookupError::Absent`], [`LookupError::BindingMismatch`] or [`LookupError::Unavailable`].
pub fn redeem_code(
    code: &str,
    client_id: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<AuthorizationCode, LookupError> {
    let payload = redis_getdel(
        "authorization_code",
        &hashed_key("oidc:authorization-code", code),
    )
    .map_err(LookupError::Unavailable)?
    .ok_or(LookupError::Absent)?;
    let payload: AuthorizationCode = serde_json::from_str(&payload).map_err(|error| {
        LookupError::Unavailable(anyhow::anyhow!("corrupt authorization code: {error}"))
    })?;
    if payload.client_id != client_id
        || !sesame_common::oidc_client::redirect_uri_matches(&payload.redirect_uri, redirect_uri)
        || !verify_pkce_s256(code_verifier, &payload.code_challenge)
    {
        tracing::warn!("OIDC authorization code binding mismatch; code burned");
        return Err(LookupError::BindingMismatch);
    }
    Ok(payload)
}

/// [`redeem_code`] without the reason.
pub fn redeem_authorization_code(
    code: &str,
    client_id: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Option<AuthorizationCode> {
    redeem_code(code, client_id, redirect_uri, code_verifier).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sesame_common::oidc_client::{ClientPolicy, ClientType, TokenEndpointAuthMethod};

    fn binding() -> ClientBinding {
        ClientBinding {
            client_id: "client".to_string(),
            tenant_id: "tenant".to_string(),
            portal: "portal".to_string(),
            application_id: "portal".to_string(),
            authority_class: "tenant".to_string(),
            policy: ClientPolicy {
                client_type: ClientType::Public,
                token_endpoint_auth_method: TokenEndpointAuthMethod::None,
                pkce_s256_required: true,
                grants: vec![
                    "authorization_code".to_string(),
                    "refresh_token".to_string(),
                ],
                response_types: vec!["code".to_string()],
                scopes: vec![
                    "openid".to_string(),
                    "profile".to_string(),
                    "email".to_string(),
                ],
                audiences: vec!["sesame-idam".to_string()],
            },
            redirect_uris: vec!["https://client.example/callback".to_string()],
            post_logout_redirect_uris: vec![],
            initiate_login_uri: None,
        }
    }

    fn challenge(verifier: &str) -> String {
        URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
    }

    #[test]
    fn validates_standard_authorization_code_request() {
        let verifier = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";
        let session = validate_authorization_request(
            &binding(),
            "code",
            "https://CLIENT.example:443/callback",
            "state-0123456789abcdef",
            "nonce-0123456789abcdef",
            "profile openid",
            &challenge(verifier),
            "S256",
        )
        .expect("valid request");
        assert_eq!(session.tenant_id, "tenant");
        assert_eq!(session.scopes, ["openid", "profile"]);
        assert!(verify_pkce_s256(verifier, &session.code_challenge));
    }

    #[test]
    fn rejects_redirect_scope_and_pkce_confusion() {
        let verifier = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";
        assert_eq!(
            validate_authorization_request(
                &binding(),
                "code",
                "https://client.example/callback/extra",
                "state-0123456789abcdef",
                "nonce-0123456789abcdef",
                "openid",
                &challenge(verifier),
                "S256",
            ),
            Err(AuthorizationError::InvalidRedirectUri)
        );
        assert_eq!(
            validate_authorization_request(
                &binding(),
                "code",
                "https://client.example/callback",
                "state-0123456789abcdef",
                "nonce-0123456789abcdef",
                "openid admin",
                &challenge(verifier),
                "S256",
            ),
            Err(AuthorizationError::InvalidScope)
        );
        assert!(validate_authorization_request(
            &binding(),
            "code",
            "https://client.example/callback",
            "state-0123456789abcdef",
            "nonce-0123456789abcdef",
            "openid",
            verifier,
            "plain",
        )
        .is_err());
    }

    #[test]
    fn changed_verifier_fails() {
        let verifier = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";
        assert!(!verify_pkce_s256(
            "changed-abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
            &challenge(verifier)
        ));
    }

    #[test]
    fn rejects_short_state_nonce_and_unsupported_response_type() {
        let verifier = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";
        let ch = challenge(verifier);
        assert!(matches!(
            validate_authorization_request(
                &binding(),
                "token",
                "https://client.example/callback",
                "state-0123456789abcdef",
                "nonce-0123456789abcdef",
                "openid",
                &ch,
                "S256",
            ),
            Err(AuthorizationError::UnsupportedResponseType)
        ));
        assert!(matches!(
            validate_authorization_request(
                &binding(),
                "code",
                "https://client.example/callback",
                "short",
                "nonce-0123456789abcdef",
                "openid",
                &ch,
                "S256",
            ),
            Err(AuthorizationError::InvalidRequest(_))
        ));
        assert!(matches!(
            validate_authorization_request(
                &binding(),
                "code",
                "https://client.example/callback",
                "state-0123456789abcdef",
                "short",
                "openid",
                &ch,
                "S256",
            ),
            Err(AuthorizationError::InvalidRequest(_))
        ));
    }

    #[test]
    fn oauth_error_codes_are_standards_compatible() {
        assert_eq!(
            AuthorizationError::InvalidClient.oauth_code(),
            "invalid_client"
        );
        assert_eq!(
            AuthorizationError::InvalidRedirectUri.oauth_code(),
            "invalid_request"
        );
        assert_eq!(
            AuthorizationError::UnsupportedResponseType.oauth_code(),
            "unsupported_response_type"
        );
        assert_eq!(
            AuthorizationError::InvalidScope.oauth_code(),
            "invalid_scope"
        );
        assert_eq!(
            AuthorizationError::ServerUnavailable.oauth_code(),
            "temporarily_unavailable"
        );
        assert_eq!(
            AuthorizationError::InvalidRequest("x").oauth_code(),
            "invalid_request"
        );
    }

    #[test]
    fn authorization_request_outlives_a_page_left_open() {
        // A login page left open for half an hour still completes; the code stays short-lived.
        assert_eq!(AUTHORIZATION_SESSION_TTL_SECS, 1800);
        assert_eq!(AUTHORIZATION_CODE_TTL_SECS, 60);
    }

    #[test]
    fn ttl_reply_maps_to_remaining_seconds() {
        assert_eq!(remaining_from_ttl_reply(-2), None);
        assert_eq!(remaining_from_ttl_reply(0), None);
        assert_eq!(remaining_from_ttl_reply(1), Some(1));
        assert_eq!(remaining_from_ttl_reply(1799), Some(1799));
        assert_eq!(
            remaining_from_ttl_reply(-1),
            Some(AUTHORIZATION_SESSION_TTL_SECS)
        );
    }

    #[test]
    fn rejects_missing_openid_scope() {
        let verifier = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";
        assert_eq!(
            validate_authorization_request(
                &binding(),
                "code",
                "https://client.example/callback",
                "state-0123456789abcdef",
                "nonce-0123456789abcdef",
                "profile email",
                &challenge(verifier),
                "S256",
            ),
            Err(AuthorizationError::InvalidScope)
        );
    }
}
