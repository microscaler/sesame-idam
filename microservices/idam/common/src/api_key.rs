//! Static API-key security schemes (`ApiKeyHeader`, `PlatformServiceAuth`, …) for BRRTRouter services.
//!
//! A key comes only from configuration: `security.api_keys.<scheme>.key` in `config.yaml`, or the environment
//! variable `SESAME_API_KEY_<SCHEME>` (scheme name upper-cased, non-alphanumerics as `_`); the platform operator
//! scheme also reads `SESAME_PLATFORM_ADMIN_KEY`. There is no built-in fallback: a scheme without a key rejects
//! every request, and a key that is empty, shorter than [`MIN_KEY_LEN`] or a known development value stops the
//! service at start-up. Comparison is constant-time.

use std::sync::Arc;

use brrtrouter::security::{SecurityProvider, SecurityRequest};
use brrtrouter::server::AppService;
use brrtrouter::spec::SecurityScheme;

/// Shortest key accepted (characters).
pub const MIN_KEY_LEN: usize = 32;
/// Values that were development fallbacks and must never authenticate anything.
const KNOWN_DEV_KEYS: &[&str] = &["test123", "dev-platform-admin"];
/// The platform operator scheme, which also reads [`PLATFORM_ADMIN_KEY_ENV`].
pub const PLATFORM_SCHEME: &str = "PlatformServiceAuth";
/// Environment variable for the platform operator key.
pub const PLATFORM_ADMIN_KEY_ENV: &str = "SESAME_PLATFORM_ADMIN_KEY";

/// Compares a presented key with the configured one in constant time; with no key configured, rejects all.
pub struct StaticApiKeyProvider {
    key: Option<String>,
}

impl StaticApiKeyProvider {
    /// A provider for `key`; `None` rejects every request.
    #[must_use]
    pub fn new(key: Option<String>) -> Self {
        Self { key }
    }
}

fn equal(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let diff = a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y));
    std::hint::black_box(diff) == 0
}

impl SecurityProvider for StaticApiKeyProvider {
    fn validate(&self, scheme: &SecurityScheme, _scopes: &[String], req: &SecurityRequest) -> bool {
        let Some(key) = self.key.as_deref() else {
            return false;
        };
        let SecurityScheme::ApiKey { name, location, .. } = scheme else {
            return false;
        };
        let presented = match location.as_str() {
            "header" => req.get_header(name),
            "query" => req.get_query(name),
            "cookie" => req.get_cookie(name),
            _ => None,
        };
        presented.is_some_and(|v| !v.is_empty() && equal(v.as_bytes(), key.as_bytes()))
    }
}

/// The environment variable for a scheme's key: `SESAME_API_KEY_<SCHEME>`.
#[must_use]
pub fn env_var_for(scheme_name: &str) -> String {
    let upper: String = scheme_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    format!("SESAME_API_KEY_{upper}")
}

/// Check a key's strength.
///
/// # Errors
///
/// A message naming the scheme (never the key) for an empty, short or known development key.
pub fn check_key(scheme_name: &str, key: &str) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() {
        return Err(format!("API key for {scheme_name} is empty"));
    }
    if KNOWN_DEV_KEYS.contains(&key) {
        return Err(format!("API key for {scheme_name} is a development value"));
    }
    if key.chars().count() < MIN_KEY_LEN {
        return Err(format!(
            "API key for {scheme_name} is shorter than {MIN_KEY_LEN} characters"
        ));
    }
    Ok(())
}

/// The key for `scheme_name`: `configured` (from `config.yaml`) first, then the environment.
///
/// # Errors
///
/// From [`check_key`] when a key is present but unacceptable.
pub fn resolve_key(
    scheme_name: &str,
    configured: Option<&str>,
    env: impl Fn(&str) -> Option<String>,
) -> Result<Option<String>, String> {
    let mut found = configured.map(str::to_string);
    if found.is_none() {
        found = env(&env_var_for(scheme_name));
    }
    if found.is_none() && scheme_name == PLATFORM_SCHEME {
        found = env(PLATFORM_ADMIN_KEY_ENV);
    }
    match found {
        Some(key) => {
            check_key(scheme_name, &key)?;
            Ok(Some(key.trim().to_string()))
        }
        None => Ok(None),
    }
}

/// Register the provider for an API-key scheme.
///
/// # Errors
///
/// From [`resolve_key`]: the service must not start with an unacceptable key.
pub fn register_api_key_scheme(
    service: &mut AppService,
    scheme_name: &str,
    configured: Option<&str>,
) -> Result<(), String> {
    let key = resolve_key(scheme_name, configured, |name| std::env::var(name).ok())?;
    match &key {
        Some(k) => println!(
            "[auth] register StaticApiKeyProvider scheme={scheme_name} key_len={}",
            k.len()
        ),
        None => {
            println!("[auth] scheme={scheme_name} has no key configured: every request is rejected")
        }
    }
    service.register_security_provider(scheme_name, Arc::new(StaticApiKeyProvider::new(key)));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = "k3y-material-long-enough-for-production-01";

    fn env_with<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| (*v).to_string())
        }
    }

    #[test]
    fn no_fallback_key_exists() {
        assert_eq!(resolve_key("ApiKeyHeader", None, env_with(&[])), Ok(None));
        assert_eq!(resolve_key(PLATFORM_SCHEME, None, env_with(&[])), Ok(None));
    }

    #[test]
    fn configured_then_scheme_env_then_platform_env() {
        assert_eq!(
            resolve_key("ApiKeyHeader", Some(GOOD), env_with(&[])),
            Ok(Some(GOOD.to_string()))
        );
        assert_eq!(
            resolve_key(
                "ApiKeyHeader",
                None,
                env_with(&[("SESAME_API_KEY_APIKEYHEADER", GOOD)])
            ),
            Ok(Some(GOOD.to_string()))
        );
        assert_eq!(
            resolve_key(
                PLATFORM_SCHEME,
                None,
                env_with(&[(PLATFORM_ADMIN_KEY_ENV, GOOD)])
            ),
            Ok(Some(GOOD.to_string()))
        );
        assert_eq!(
            env_var_for("Tenant-CS.Auth"),
            "SESAME_API_KEY_TENANT_CS_AUTH"
        );
    }

    #[test]
    fn weak_keys_stop_start_up_without_naming_the_key() {
        for bad in ["", "   ", "test123", "dev-platform-admin", "short-key"] {
            let err = resolve_key("ApiKeyHeader", Some(bad), env_with(&[])).expect_err(bad);
            assert!(err.contains("ApiKeyHeader"));
            if !bad.trim().is_empty() {
                assert!(
                    !err.contains(bad.trim()),
                    "error must not echo the key: {err}"
                );
            }
        }
    }

    #[test]
    fn comparison() {
        assert!(equal(b"abc", b"abc"));
        assert!(!equal(b"abc", b"abd"));
        assert!(!equal(b"abc", b"abcd"));
    }
}
