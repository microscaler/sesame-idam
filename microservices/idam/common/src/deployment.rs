//! Deployment settings every service reads, and the start-up check that refuses to run without them.
//!
//! A deployment names its issuer and public URLs and mounts its signing keys. Nothing here has a production
//! default: [`check_startup`] stops a service whose settings are missing, point at a development or example host,
//! or are not `https`, and a signing service without a configured key. Development (Tilt, the dev profile, tests)
//! sets [`DEV_MODE_ENV`]`=1`, which turns the check into warnings and lets the accessors fall back to the
//! development values below; those values are never used in a deployment that passed the check.

use std::fmt::Write as _;

/// `1` (or `true`) in development only: allows the development fallbacks and an ephemeral signing key.
pub const DEV_MODE_ENV: &str = "SESAME_DEV_MODE";
/// The token issuer (`iss`), also the OIDC discovery issuer.
pub const ISSUER_ENV: &str = "SESAME_JWT_ISSUER";
/// Public base URL of the hosted sign-in (authorize) surface.
pub const AUTH_PUBLIC_URL_ENV: &str = "SESAME_AUTH_PUBLIC_URL";
/// Public base URL of the API surface (token, userinfo, JWKS).
pub const API_PUBLIC_URL_ENV: &str = "SESAME_API_PUBLIC_URL";
/// The hosted sign-in page the authorize endpoint redirects to.
pub const HOSTED_AUTH_URL_ENV: &str = "OIDC_HOSTED_AUTH_URL";

/// Development issuer (an example host; never valid outside development).
pub const DEV_ISSUER: &str = "https://idam.example.com";
/// Development hosted sign-in base URL.
pub const DEV_AUTH_PUBLIC_URL: &str = "https://auth.sesameidentity.dev.local";
/// Development API base URL.
pub const DEV_API_PUBLIC_URL: &str = "https://api.sesameidentity.dev.local";

/// Host suffixes that mark a development or documentation value.
const DEV_HOST_SUFFIXES: &[&str] = &[
    ".local",
    ".localhost",
    ".test",
    ".example",
    ".invalid",
    "example.com",
    "example.org",
    "example.net",
];

/// Whether [`DEV_MODE_ENV`] is set to `1` or `true`.
#[must_use]
pub fn dev_mode() -> bool {
    dev_mode_from(std::env::var(DEV_MODE_ENV).ok().as_deref())
}

fn dev_mode_from(value: Option<&str>) -> bool {
    value.is_some_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true"))
}

fn env_or(name: &str, dev_default: &str) -> String {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => v.trim().trim_end_matches('/').to_string(),
        _ => dev_default.to_string(),
    }
}

/// The token issuer: [`ISSUER_ENV`], else [`DEV_ISSUER`] (reachable only in development; see [`check_startup`]).
#[must_use]
pub fn issuer() -> String {
    env_or(ISSUER_ENV, DEV_ISSUER)
}

/// Audiences minted into every access token Sesame issues (Gate A6), at login and at refresh alike:
/// [`ISSUE_AUDIENCES_ENV`] (comma-separated), else [`DEFAULT_ISSUE_AUDIENCES`] — the platform audience plus every
/// consumer service, so a browser token is accepted by each service's own `aud` check. Login and refresh must agree:
/// a refreshed token narrower than the login one is refused by the services and signs the user out.
#[must_use]
pub fn issue_audiences() -> Vec<String> {
    parse_audiences(std::env::var(ISSUE_AUDIENCES_ENV).ok().as_deref())
}

/// Environment variable naming the audiences minted into access tokens.
pub const ISSUE_AUDIENCES_ENV: &str = "JWT_ISSUE_AUDIENCES";

/// Audiences used when [`ISSUE_AUDIENCES_ENV`] is unset or empty.
pub const DEFAULT_ISSUE_AUDIENCES: [&str; 7] = [
    "sesame-idam",
    "identity-login",
    "identity-session",
    "authz-core",
    "org-mgmt",
    "identity-user-mgmt",
    "api-keys",
];

fn parse_audiences(raw: Option<&str>) -> Vec<String> {
    let listed: Vec<String> = raw
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    if listed.is_empty() {
        DEFAULT_ISSUE_AUDIENCES
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    } else {
        listed
    }
}

/// Public base URL of the sign-in surface: [`AUTH_PUBLIC_URL_ENV`], else [`DEV_AUTH_PUBLIC_URL`].
#[must_use]
pub fn auth_public_url() -> String {
    env_or(AUTH_PUBLIC_URL_ENV, DEV_AUTH_PUBLIC_URL)
}

/// Public base URL of the API surface: [`API_PUBLIC_URL_ENV`], else [`DEV_API_PUBLIC_URL`].
#[must_use]
pub fn api_public_url() -> String {
    env_or(API_PUBLIC_URL_ENV, DEV_API_PUBLIC_URL)
}

/// The hosted sign-in page: [`HOSTED_AUTH_URL_ENV`], else `<auth public URL>/authorize`.
#[must_use]
pub fn hosted_auth_url() -> String {
    match std::env::var(HOSTED_AUTH_URL_ENV) {
        Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => format!("{}/authorize", auth_public_url()),
    }
}

/// What a service needs from its deployment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Requirements {
    /// Validates or issues tokens: needs [`ISSUER_ENV`].
    pub issuer: bool,
    /// Builds public links or discovery: needs [`AUTH_PUBLIC_URL_ENV`] and [`API_PUBLIC_URL_ENV`].
    pub public_urls: bool,
    /// Signs access tokens or publishes their JWKS: needs the shared keyset (or the legacy key pair).
    pub signing_key: bool,
    /// Signs or publishes ID tokens: needs `SESAME_ID_TOKEN_KEYS` (RS256, ADR-015).
    pub id_token_keys: bool,
}

impl Requirements {
    /// A service that only validates tokens.
    pub const VALIDATOR: Self = Self {
        issuer: true,
        public_urls: false,
        signing_key: false,
        id_token_keys: false,
    };
    /// A service that issues tokens or publishes their keys (identity-login, identity-session).
    pub const ISSUER: Self = Self {
        issuer: true,
        public_urls: true,
        signing_key: true,
        id_token_keys: true,
    };
}

fn is_dev_host(host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    host == "localhost"
        || host.parse::<std::net::IpAddr>().is_ok()
        || DEV_HOST_SUFFIXES
            .iter()
            .any(|s| host == s.trim_start_matches('.') || host.ends_with(s))
}

/// Why a public URL setting is unacceptable outside development, or `None`.
fn url_problem(name: &str, value: Option<&str>) -> Option<String> {
    let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return Some(format!("{name} is not set"));
    };
    let Ok(url) = url::Url::parse(value) else {
        return Some(format!("{name} is not a URL"));
    };
    if url.scheme() != "https" {
        return Some(format!("{name} must be https"));
    }
    match url.host_str() {
        Some(host) if !is_dev_host(host) => None,
        _ => Some(format!("{name} names a development or example host")),
    }
}

/// Every problem with the deployment settings, read through `env`.
#[must_use]
pub fn problems(req: Requirements, env: &dyn Fn(&str) -> Option<String>) -> Vec<String> {
    let set = |name: &str| env(name).filter(|v| !v.trim().is_empty());
    let mut out = Vec::new();
    if req.issuer {
        out.extend(url_problem(ISSUER_ENV, set(ISSUER_ENV).as_deref()));
    }
    if req.public_urls {
        for name in [AUTH_PUBLIC_URL_ENV, API_PUBLIC_URL_ENV] {
            out.extend(url_problem(name, set(name).as_deref()));
        }
        if let Some(hosted) = set(HOSTED_AUTH_URL_ENV) {
            out.extend(url_problem(HOSTED_AUTH_URL_ENV, Some(&hosted)));
        }
    }
    if req.signing_key {
        let keyset = set(crate::jwt::KEY_SOURCE_ENV)
            .is_some_and(|s| s.trim().eq_ignore_ascii_case("file"))
            && set(crate::jwt::KEYSET_FILE_ENV).is_some();
        let pair = set(crate::jwt::SIGNING_KEY_ENV).is_some()
            && set(crate::jwt::SIGNING_KID_ENV).is_some();
        if !keyset && !pair {
            out.push(format!(
                "no signing key: set {}=file and {}",
                crate::jwt::KEY_SOURCE_ENV,
                crate::jwt::KEYSET_FILE_ENV
            ));
        }
    }
    if req.id_token_keys && set(crate::jwt::ID_TOKEN_KEYS_ENV).is_none() {
        out.push(format!(
            "{} is not set (ID tokens are RS256)",
            crate::jwt::ID_TOKEN_KEYS_ENV
        ));
    }
    out
}

/// Check the deployment settings at start-up.
///
/// # Errors
///
/// Outside development, one message listing every problem (setting names only, never values). In development the
/// problems are logged as warnings and the check passes.
pub fn check_startup(service: &str, req: Requirements) -> Result<(), String> {
    let found = problems(req, &|name| std::env::var(name).ok());
    if found.is_empty() {
        return Ok(());
    }
    if dev_mode() {
        for problem in &found {
            tracing::warn!(
                service,
                problem,
                "{DEV_MODE_ENV}=1: development fallback in use"
            );
        }
        return Ok(());
    }
    let mut msg = format!("{service}: refusing to start without its deployment settings");
    for problem in &found {
        let _ = write!(msg, "; {problem}");
    }
    let _ = write!(
        msg,
        " (development only: {DEV_MODE_ENV}=1 allows the development fallbacks)"
    );
    Err(msg)
}

/// [`check_startup`], exiting the process on failure.
pub fn require_or_exit(service: &str, req: Requirements) {
    if let Err(msg) = check_startup(service, req) {
        eprintln!("[startup][error] {msg}");
        tracing::error!("{msg}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| (*v).to_string())
        }
    }

    #[test]
    fn nothing_set_is_every_problem() {
        let found = problems(Requirements::ISSUER, &env(&[]));
        assert_eq!(found.len(), 5, "{found:?}");
        assert!(problems(Requirements::default(), &env(&[])).is_empty());
    }

    #[test]
    fn complete_production_settings_pass() {
        let good = [
            (ISSUER_ENV, "https://id.acme.io"),
            (AUTH_PUBLIC_URL_ENV, "https://auth.acme.io"),
            (API_PUBLIC_URL_ENV, "https://api.acme.io"),
            ("KEY_SOURCE", "file"),
            (
                "SESAME_SIGNING_KEYSET_FILE",
                "/etc/sesame/keys/signing-keyset.json",
            ),
            ("SESAME_ID_TOKEN_KEYS", "/etc/sesame/id-token/keys.json"),
        ];
        assert_eq!(
            problems(Requirements::ISSUER, &env(&good)),
            Vec::<String>::new()
        );
    }

    #[test]
    fn development_and_plain_http_hosts_are_refused() {
        for bad in [
            "https://idam.example.com",
            "https://id.sesameidentity.dev.local",
            "https://localhost:8443",
            "https://127.0.0.1",
            "http://id.acme.io",
            "id.acme.io",
        ] {
            let found = problems(Requirements::VALIDATOR, &env(&[(ISSUER_ENV, bad)]));
            assert_eq!(found.len(), 1, "{bad}: {found:?}");
            assert!(!found[0].contains(bad), "never echo the value");
        }
    }

    #[test]
    fn keyset_source_without_file_is_no_key() {
        let found = problems(
            Requirements {
                signing_key: true,
                ..Requirements::default()
            },
            &env(&[("KEY_SOURCE", "file")]),
        );
        assert_eq!(found.len(), 1);
        let pair = problems(
            Requirements {
                signing_key: true,
                ..Requirements::default()
            },
            &env(&[
                ("SESAME_JWT_SIGNING_KEY_PKCS8_B64", "x"),
                ("SESAME_JWT_SIGNING_KID", "k"),
            ]),
        );
        assert!(pair.is_empty());
    }

    #[test]
    fn dev_mode_values() {
        assert!(dev_mode_from(Some("1")));
        assert!(dev_mode_from(Some("TRUE")));
        assert!(!dev_mode_from(Some("0")));
        assert!(!dev_mode_from(Some("")));
        assert!(!dev_mode_from(None));
    }
}

#[cfg(test)]
mod issue_audience_tests {
    use super::{parse_audiences, DEFAULT_ISSUE_AUDIENCES};

    #[test]
    fn unset_or_blank_means_every_service() {
        for raw in [None, Some(""), Some(" , ")] {
            let aud = parse_audiences(raw);
            assert_eq!(aud.len(), DEFAULT_ISSUE_AUDIENCES.len());
            assert!(aud.iter().any(|a| a == "identity-session"));
        }
    }

    #[test]
    fn configured_list_is_trimmed() {
        assert_eq!(parse_audiences(Some(" a, b ,,c")), vec!["a", "b", "c"]);
    }
}
