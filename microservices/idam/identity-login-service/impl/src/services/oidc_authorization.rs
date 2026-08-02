//! OIDC Authorization Code + PKCE session and code lifecycle.

use anyhow::{Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use redis::Commands;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::services::client_registry::ClientBinding;

const AUTHORIZATION_SESSION_TTL_SECS: u64 = 300;
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

fn redis_connection() -> Result<redis::Connection> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let client = redis::Client::open(redis_url.as_str())?;
    client.get_connection().context("connect to Redis")
}

fn opaque_value() -> String {
    let mut bytes = [0_u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn hashed_key(prefix: &str, value: &str) -> String {
    format!("{prefix}:{}", hex::encode(Sha256::digest(value.as_bytes())))
}

pub fn store_authorization_session(session: &AuthorizationSession) -> Result<String> {
    let request_id = opaque_value();
    let payload = serde_json::to_string(session).context("serialize authorization session")?;
    let mut connection = redis_connection()?;
    connection
        .set_ex::<_, _, ()>(
            hashed_key("oidc:authorization-request", &request_id),
            payload,
            AUTHORIZATION_SESSION_TTL_SECS,
        )
        .context("store authorization session")?;
    Ok(request_id)
}

pub fn consume_authorization_session(request_id: &str) -> Option<AuthorizationSession> {
    let mut connection = redis_connection().ok()?;
    let payload: Option<String> = redis::cmd("GETDEL")
        .arg(hashed_key("oidc:authorization-request", request_id))
        .query(&mut connection)
        .ok()?;
    serde_json::from_str(&payload?).ok()
}

pub fn mint_authorization_code(code: &AuthorizationCode) -> Result<String> {
    let value = opaque_value();
    let payload = serde_json::to_string(code).context("serialize authorization code")?;
    let mut connection = redis_connection()?;
    connection
        .set_ex::<_, _, ()>(
            hashed_key("oidc:authorization-code", &value),
            payload,
            AUTHORIZATION_CODE_TTL_SECS,
        )
        .context("store authorization code")?;
    Ok(value)
}

pub fn redeem_authorization_code(
    code: &str,
    client_id: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Option<AuthorizationCode> {
    let mut connection = redis_connection().ok()?;
    let payload: Option<String> = redis::cmd("GETDEL")
        .arg(hashed_key("oidc:authorization-code", code))
        .query(&mut connection)
        .ok()?;
    let payload: AuthorizationCode = serde_json::from_str(&payload?).ok()?;
    if payload.client_id != client_id
        || !sesame_common::oidc_client::redirect_uri_matches(&payload.redirect_uri, redirect_uri)
        || !verify_pkce_s256(code_verifier, &payload.code_challenge)
    {
        tracing::warn!("OIDC authorization code binding mismatch; code burned");
        return None;
    }
    Some(payload)
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
}
