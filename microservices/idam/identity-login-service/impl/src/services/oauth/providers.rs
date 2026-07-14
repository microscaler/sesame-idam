//! Provider-specific authorize URLs, token exchange, and profile fetch.

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use sesame_common::http::{fetch_get, fetch_post, HttpFetchOptions};

use super::config::{ProviderCredentials, SupportedProvider};

/// Normalized identity from an OAuth provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderProfile {
    pub provider_user_id: String,
    pub email: String,
    pub email_verified: bool,
}

/// Build the provider authorization URL (browser redirect target).
#[must_use]
pub fn build_authorize_url(
    provider: SupportedProvider,
    creds: &ProviderCredentials,
    redirect_uri: &str,
    state: &str,
    scope: Option<&str>,
) -> String {
    let default_scope = match provider {
        SupportedProvider::Google => "openid email profile",
        SupportedProvider::Microsoft => "openid email profile User.Read",
    };
    let scope = scope.unwrap_or(default_scope);

    let base = match provider {
        SupportedProvider::Google => "https://accounts.google.com/o/oauth2/v2/auth",
        SupportedProvider::Microsoft => {
            "https://login.microsoftonline.com/common/oauth2/v2.0/authorize"
        }
    };

    format!(
        "{base}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&prompt=select_account",
        url_encode(&creds.client_id),
        url_encode(redirect_uri),
        url_encode(scope),
        url_encode(state),
    )
}

/// Exchange an authorization code for tokens and resolve the user's profile.
///
/// # Errors
///
/// Returns an error when the provider rejects the code or the profile cannot be loaded.
pub fn exchange_code(
    provider: SupportedProvider,
    creds: &ProviderCredentials,
    code: &str,
    redirect_uri: &str,
) -> Result<ProviderProfile> {
    let token_body = match provider {
        SupportedProvider::Google => exchange_google_token(creds, code, redirect_uri)?,
        SupportedProvider::Microsoft => exchange_microsoft_token(creds, code, redirect_uri)?,
    };

    let access_token = token_body
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("provider token response missing access_token"))?;

    match provider {
        SupportedProvider::Google => fetch_google_profile(access_token),
        SupportedProvider::Microsoft => fetch_microsoft_profile(access_token),
    }
}

fn exchange_google_token(
    creds: &ProviderCredentials,
    code: &str,
    redirect_uri: &str,
) -> Result<serde_json::Value> {
    let body = form_body(&[
        ("code", code),
        ("client_id", &creds.client_id),
        ("client_secret", &creds.client_secret),
        ("redirect_uri", redirect_uri),
        ("grant_type", "authorization_code"),
    ]);
    post_form("https://oauth2.googleapis.com/token", &body)
}

fn exchange_microsoft_token(
    creds: &ProviderCredentials,
    code: &str,
    redirect_uri: &str,
) -> Result<serde_json::Value> {
    let body = form_body(&[
        ("code", code),
        ("client_id", &creds.client_id),
        ("client_secret", &creds.client_secret),
        ("redirect_uri", redirect_uri),
        ("grant_type", "authorization_code"),
        ("scope", "openid email profile User.Read"),
    ]);
    post_form(
        "https://login.microsoftonline.com/common/oauth2/v2.0/token",
        &body,
    )
}

fn fetch_google_profile(access_token: &str) -> Result<ProviderProfile> {
    let url = "https://openidconnect.googleapis.com/v1/userinfo";
    let json = get_bearer_json(url, access_token)?;
    let sub = json
        .get("sub")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("google profile missing sub"))?;
    let email = json
        .get("email")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("google profile missing email"))?;
    let email_verified = json
        .get("email_verified")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    Ok(ProviderProfile {
        provider_user_id: sub.to_string(),
        email: email.to_ascii_lowercase(),
        email_verified,
    })
}

fn fetch_microsoft_profile(access_token: &str) -> Result<ProviderProfile> {
    let url = "https://graph.microsoft.com/v1.0/me";
    let json = get_bearer_json(url, access_token)?;
    let sub = json
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("microsoft profile missing id"))?;
    let email = json
        .get("mail")
        .and_then(|v| v.as_str())
        .or_else(|| json.get("userPrincipalName").and_then(|v| v.as_str()))
        .ok_or_else(|| anyhow!("microsoft profile missing email"))?;
    Ok(ProviderProfile {
        provider_user_id: sub.to_string(),
        email: email.to_ascii_lowercase(),
        email_verified: true,
    })
}

fn post_form(url: &str, body: &str) -> Result<serde_json::Value> {
    let options = HttpFetchOptions {
        timeout: Duration::from_secs(10),
        max_body_bytes: 64 * 1024,
        extra_headers: vec![(
            "content-type".to_string(),
            "application/x-www-form-urlencoded".to_string(),
        )],
    };
    let (status, bytes) = fetch_post(url, body.as_bytes(), &options)
        .map_err(|e| anyhow!("provider token request failed: {e}"))?;
    if !(200..300).contains(&status) {
        let text = String::from_utf8_lossy(&bytes);
        return Err(anyhow!("provider token HTTP {status}: {text}"));
    }
    serde_json::from_slice(&bytes).context("parse provider token JSON")
}

fn get_bearer_json(url: &str, access_token: &str) -> Result<serde_json::Value> {
    let options = HttpFetchOptions {
        timeout: Duration::from_secs(10),
        max_body_bytes: 64 * 1024,
        extra_headers: vec![(
            "authorization".to_string(),
            format!("Bearer {access_token}"),
        )],
    };
    let (status, bytes) =
        fetch_get(url, &options).map_err(|e| anyhow!("provider profile request failed: {e}"))?;
    if !(200..300).contains(&status) {
        let text = String::from_utf8_lossy(&bytes);
        return Err(anyhow!("provider profile HTTP {status}: {text}"));
    }
    serde_json::from_slice(&bytes).context("parse provider profile JSON")
}

fn form_body(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                use std::fmt::Write as _;
                let _ = write!(out, "%{b:02X}");
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn google_authorize_url_contains_required_params() {
        let creds = ProviderCredentials {
            client_id: "cid".to_string(),
            client_secret: "secret".to_string(),
        };
        let url = build_authorize_url(
            SupportedProvider::Google,
            &creds,
            "http://localhost:7174/oauth/callback",
            "state-xyz",
            None,
        );
        assert!(url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?"));
        assert!(url.contains("client_id=cid"));
        assert!(url.contains("state=state-xyz"));
        assert!(url.contains("redirect_uri="));
    }
}
