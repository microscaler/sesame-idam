//! Language-neutral OIDC relying-party policy primitives.

use std::collections::HashSet;
use std::fmt;

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::{Host, Url};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientType {
    Public,
    Confidential,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenEndpointAuthMethod {
    None,
    ClientSecretBasic,
    ClientSecretPost,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientPolicy {
    pub client_type: ClientType,
    pub token_endpoint_auth_method: TokenEndpointAuthMethod,
    pub pkce_s256_required: bool,
    pub grants: Vec<String>,
    pub response_types: Vec<String>,
    pub scopes: Vec<String>,
    pub audiences: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ClientPolicyError {
    #[error("redirect URI is invalid")]
    InvalidRedirectUri,
    #[error("redirect URI must use HTTPS")]
    InsecureRedirectUri,
    #[error("loopback HTTP redirect URI requires an explicit port")]
    LoopbackPortRequired,
    #[error("public clients cannot use a client secret")]
    PublicClientSecretForbidden,
    #[error("public clients require PKCE S256")]
    PublicClientPkceRequired,
    #[error("confidential clients require client authentication")]
    ConfidentialClientAuthenticationRequired,
    #[error("unsupported grant, response type, scope, or audience policy")]
    UnsupportedCapability,
}

impl ClientPolicy {
    pub fn validate(&self) -> Result<(), ClientPolicyError> {
        if self.grants.is_empty()
            || self.response_types.is_empty()
            || self.scopes.iter().all(|scope| scope != "openid")
            || self.audiences.is_empty()
            || self
                .grants
                .iter()
                .any(|grant| !matches!(grant.as_str(), "authorization_code" | "refresh_token"))
            || self
                .response_types
                .iter()
                .any(|response_type| response_type != "code")
            || self
                .scopes
                .iter()
                .any(|scope| !is_valid_capability_name(scope))
            || self
                .audiences
                .iter()
                .any(|audience| audience.trim().is_empty())
        {
            return Err(ClientPolicyError::UnsupportedCapability);
        }

        match self.client_type {
            ClientType::Public => {
                if self.token_endpoint_auth_method != TokenEndpointAuthMethod::None {
                    return Err(ClientPolicyError::PublicClientSecretForbidden);
                }
                if !self.pkce_s256_required {
                    return Err(ClientPolicyError::PublicClientPkceRequired);
                }
            }
            ClientType::Confidential => {
                if self.token_endpoint_auth_method == TokenEndpointAuthMethod::None {
                    return Err(ClientPolicyError::ConfidentialClientAuthenticationRequired);
                }
            }
        }
        Ok(())
    }
}

fn is_valid_capability_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b':' | b'.'))
}

pub fn normalize_redirect_uri(raw: &str) -> Result<String, ClientPolicyError> {
    let raw = raw.trim();
    let lower = raw.to_ascii_lowercase();
    if raw.is_empty()
        || raw.contains('*')
        || lower.contains("%2e")
        || lower.contains("%2f")
        || lower.contains("%5c")
    {
        return Err(ClientPolicyError::InvalidRedirectUri);
    }

    let mut uri = Url::parse(raw).map_err(|_| ClientPolicyError::InvalidRedirectUri)?;
    if uri.cannot_be_a_base()
        || uri.fragment().is_some()
        || !uri.username().is_empty()
        || uri.password().is_some()
        || uri.host().is_none()
    {
        return Err(ClientPolicyError::InvalidRedirectUri);
    }

    let is_loopback = match uri.host() {
        Some(Host::Domain(host)) => host.eq_ignore_ascii_case("localhost"),
        Some(Host::Ipv4(address)) => address.is_loopback(),
        Some(Host::Ipv6(address)) => address.is_loopback(),
        None => false,
    };

    match uri.scheme() {
        "https" => {}
        "http" if is_loopback => {
            if uri.port().is_none() {
                return Err(ClientPolicyError::LoopbackPortRequired);
            }
        }
        "http" => return Err(ClientPolicyError::InsecureRedirectUri),
        _ => return Err(ClientPolicyError::InvalidRedirectUri),
    }

    let mut query_names = HashSet::new();
    for (name, _) in uri.query_pairs() {
        if !query_names.insert(name.into_owned()) {
            return Err(ClientPolicyError::InvalidRedirectUri);
        }
    }

    uri.set_fragment(None);
    Ok(uri.to_string())
}

#[must_use]
pub fn redirect_uri_matches(registered: &str, requested: &str) -> bool {
    match (
        normalize_redirect_uri(registered),
        normalize_redirect_uri(requested),
    ) {
        (Ok(registered), Ok(requested)) => registered == requested,
        _ => false,
    }
}

pub struct ClientSecret(String);

impl ClientSecret {
    #[must_use]
    pub fn generate() -> Self {
        let mut bytes = [0_u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        Self(format!("ses_{}", URL_SAFE_NO_PAD.encode(bytes)))
    }

    #[must_use]
    pub fn expose_once(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ClientSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ClientSecret([REDACTED])")
    }
}

pub fn hash_client_secret(secret: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(secret.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| format!("client secret hashing failed: {error}"))
}

#[must_use]
pub fn verify_client_secret(secret: &str, stored_hash: &str) -> bool {
    let Ok(hash) = PasswordHash::new(stored_hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(secret.as_bytes(), &hash)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redirect_uri_normalization_is_exact_and_canonical() {
        assert_eq!(
            normalize_redirect_uri("https://EXAMPLE.com:443/callback?flow=login").unwrap(),
            "https://example.com/callback?flow=login"
        );
        assert!(redirect_uri_matches(
            "https://example.com/callback?flow=login",
            "https://EXAMPLE.com:443/callback?flow=login"
        ));
        assert!(!redirect_uri_matches(
            "https://example.com/callback",
            "https://example.com/callback/extra"
        ));
    }

    #[test]
    fn redirect_uri_attack_forms_are_rejected() {
        for uri in [
            "http://example.com/callback",
            "https://*.example.com/callback",
            "https://example.com/callback#fragment",
            "https://example.com/%2e%2e/callback",
            "https://example.com/callback?a=1&a=2",
            "https://user:password@example.com/callback",
        ] {
            assert!(normalize_redirect_uri(uri).is_err(), "{uri} must fail");
        }
    }

    #[test]
    fn loopback_http_requires_an_explicit_port() {
        assert!(normalize_redirect_uri("http://127.0.0.1/callback").is_err());
        assert_eq!(
            normalize_redirect_uri("http://127.0.0.1:49152/callback").unwrap(),
            "http://127.0.0.1:49152/callback"
        );
        assert_eq!(
            normalize_redirect_uri("http://[::1]:49152/callback").unwrap(),
            "http://[::1]:49152/callback"
        );
    }

    #[test]
    fn public_clients_require_pkce_and_never_have_secrets() {
        let policy = ClientPolicy {
            client_type: ClientType::Public,
            token_endpoint_auth_method: TokenEndpointAuthMethod::None,
            pkce_s256_required: true,
            grants: vec![
                "authorization_code".to_string(),
                "refresh_token".to_string(),
            ],
            response_types: vec!["code".to_string()],
            scopes: vec!["openid".to_string(), "profile".to_string()],
            audiences: vec!["sesame-idam".to_string()],
        };
        assert!(policy.validate().is_ok());

        let invalid = ClientPolicy {
            token_endpoint_auth_method: TokenEndpointAuthMethod::ClientSecretBasic,
            ..policy
        };
        assert_eq!(
            invalid.validate(),
            Err(ClientPolicyError::PublicClientSecretForbidden)
        );
    }

    #[test]
    fn confidential_clients_require_supported_authentication() {
        let policy = ClientPolicy {
            client_type: ClientType::Confidential,
            token_endpoint_auth_method: TokenEndpointAuthMethod::ClientSecretBasic,
            pkce_s256_required: false,
            grants: vec!["authorization_code".to_string()],
            response_types: vec!["code".to_string()],
            scopes: vec!["openid".to_string()],
            audiences: vec!["sesame-idam".to_string()],
        };
        assert!(policy.validate().is_ok());
    }

    #[test]
    fn generated_secrets_are_hashed_and_redacted() {
        let secret = ClientSecret::generate();
        let plaintext = secret.expose_once().to_string();
        let hash = hash_client_secret(&plaintext).unwrap();

        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_client_secret(&plaintext, &hash));
        assert!(!verify_client_secret("wrong", &hash));
        assert_eq!(format!("{secret:?}"), "ClientSecret([REDACTED])");
    }
}
