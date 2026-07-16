//! Platform admin API key validation (`X-Platform-Admin-Key`).

use brrtrouter::typed::HttpJson;

pub const PLATFORM_ADMIN_KEY_ENV: &str = "SESAME_PLATFORM_ADMIN_KEY";
pub const PLATFORM_ADMIN_HEADER: &str = "X-Platform-Admin-Key";

/// Result of platform admin key check when key is extracted by caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformAuthError {
    Unconfigured,
    Missing,
    Invalid,
}

impl PlatformAuthError {
    #[must_use]
    pub fn http_status(&self) -> u16 {
        match self {
            Self::Unconfigured => 503,
            Self::Missing | Self::Invalid => 401,
        }
    }

    #[must_use]
    pub fn api_error(&self) -> &'static str {
        match self {
            Self::Unconfigured => "platform_auth_unconfigured",
            Self::Missing | Self::Invalid => "unauthorized",
        }
    }
}

/// Validate the platform admin key from a request header value.
pub fn validate_platform_key(presented: Option<&str>) -> Result<(), PlatformAuthError> {
    let expected = std::env::var(PLATFORM_ADMIN_KEY_ENV)
        .ok()
        .filter(|s| !s.trim().is_empty());
    let Some(expected) = expected else {
        return Err(PlatformAuthError::Unconfigured);
    };
    let Some(presented) = presented.map(str::trim).filter(|s| !s.is_empty()) else {
        return Err(PlatformAuthError::Missing);
    };
    if presented.len() != expected.len() {
        return Err(PlatformAuthError::Invalid);
    }
    let valid = presented
        .as_bytes()
        .iter()
        .zip(expected.as_bytes().iter())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b));
    if valid == 0 {
        Ok(())
    } else {
        Err(PlatformAuthError::Invalid)
    }
}

#[must_use]
pub fn platform_auth_http_error(err: &PlatformAuthError) -> HttpJson<serde_json::Value> {
    HttpJson::new(
        err.http_status(),
        serde_json::json!({
            "error": err.api_error(),
            "error_description": err.api_error(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_missing_key_when_configured() {
        std::env::set_var(PLATFORM_ADMIN_KEY_ENV, "secret-key");
        assert_eq!(validate_platform_key(None), Err(PlatformAuthError::Missing));
        std::env::remove_var(PLATFORM_ADMIN_KEY_ENV);
    }

    #[test]
    fn accepts_matching_key() {
        std::env::set_var(PLATFORM_ADMIN_KEY_ENV, "secret-key");
        assert!(validate_platform_key(Some("secret-key")).is_ok());
        std::env::remove_var(PLATFORM_ADMIN_KEY_ENV);
    }
}
