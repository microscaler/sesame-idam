//! JWKS consumer client for per-service JWT validation.
//!
//! Each service that validates JWTs fetches the JWKS from identity-session-service,
//! caches it for 5 minutes, and uses it to verify token signatures locally.
//!
//! # Validation Pipeline (RFC 9068)
//!
//! The validation pipeline MUST be executed in the exact order below:
//!
//! ```text
//! 1. Parse JOSE header              -- Extract typ, alg, kid before ANY trust decision
//! 2. Require typ = at+jwt           -- Reject type confusion (F-002) BEFORE signature check
//! 3. Require algorithm from allow   -- Reject alg: none, reject HS256, reject unexpected alg
//! 4. Choose key by kid from JWKS    -- Select public key for verification
//! 5. Verify signature               -- CRYPTOGRAPHIC TRUST DECISION POINT
//! 6. Validate iss, aud, exp, nbf    -- Claim validation (after trust is established)
//! 7. Reject if jti in local deny    -- Revocation check (optimization only)
//! 8. Compare token ver to cached    -- Version check for high-risk routes
//! 9. Evaluate local policy from     -- Authorization decision
//! 10. If high-risk route: call      -- Selective online fallback
//! ```
//!
//! # Example
//!
//! ```ignore
//! use brrtrouter::security::JwksBearerProvider;
//!
//! let provider = JwksBearerProvider::new("http://localhost:8105")
//!     .issuer("https://idam.seasame-idam.microscaler.local")
//!     .audience("authz-core.seasame-idam.microscaler.local")
//!     .leeway(60)
//!     .cache_ttl(std::time::Duration::from_secs(300));
//!
//! // Validate a token (returns decoded claims)
//! let claims = provider.validate_token("eyJ...").await?;
//! ```

use brrtrouter::security::JwksBearerProvider;

// ─── Algorithm allow-list ────────────────────────────────────────────────────

/// Algorithms allowed for JWT verification.
///
/// `EdDSA` is the default signing algorithm. `ES256` is co-default for interoperability.
pub const ALLOWED_JWT_ALGORITHMS: &[&str] = &["EdDSA", "ES256"];

/// Reject `alg: none` (RFC 8725).
#[must_use]
pub fn reject_none_algorithm(alg: &str) -> bool {
    alg.to_lowercase() == "none"
}

/// Check if an algorithm is in the allow-list.
#[must_use]
pub fn is_allowed_algorithm(alg: &str) -> bool {
    ALLOWED_JWT_ALGORITHMS.contains(&alg)
}

// ─── JWT claim validation ────────────────────────────────────────────────────

/// Validation result with error reason for metrics/alerting.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    Valid,
    InvalidTyp {
        expected: String,
        got: String,
    },
    InvalidAlgorithm {
        expected: Vec<&'static str>,
        got: String,
    },
    MissingKkid,
    KeyNotFound {
        kid: String,
    },
    InvalidSignature,
    InvalidIssuer {
        expected: Vec<String>,
        got: String,
    },
    InvalidAudience {
        expected: Vec<String>,
        got: String,
    },
    Expired {
        exp: i64,
        now: i64,
        leeway: i64,
    },
    NotBefore {
        nbf: i64,
        now: i64,
        leeway: i64,
    },
    MissingSubject,
    JtiRevoked,
}

impl ValidationResult {
    /// Return a short reason string suitable for metrics labels.
    #[must_use]
    pub fn reason(&self) -> &str {
        match self {
            ValidationResult::Valid => "valid",
            ValidationResult::InvalidTyp { .. } => "typ",
            ValidationResult::InvalidAlgorithm { .. } => "alg",
            ValidationResult::MissingKkid | ValidationResult::KeyNotFound { .. } => "kid",
            ValidationResult::InvalidSignature => "sig",
            ValidationResult::InvalidIssuer { .. } => "iss",
            ValidationResult::InvalidAudience { .. } => "aud",
            ValidationResult::Expired { .. } => "exp",
            ValidationResult::NotBefore { .. } => "nbf",
            ValidationResult::MissingSubject => "sub",
            ValidationResult::JtiRevoked => "jti",
        }
    }
}

// ─── JWKS configuration per service ──────────────────────────────────────────

/// Configuration for a single service's JWKS consumer.
///
/// Each service has its own audience and issuer configuration.
/// The JWKS URL is derived from the identity-session-service base URL.
#[derive(Debug, Clone)]
pub struct JwksServiceConfig {
    /// The base URL of identity-session-service (e.g., `http://localhost:8105`).
    pub jwks_base_url: String,

    /// Expected JWT issuer (from the `iss` claim).
    pub issuer: String,

    /// Expected JWT audience (from the `aud` claim) — exact match, no partial.
    pub audience: String,

    /// JWKS cache TTL in seconds (default: 300 = 5 minutes).
    pub cache_ttl_secs: u64,

    /// Clock skew tolerance in seconds for `exp`/`nbf` validation (default: 60).
    pub leeway_secs: i64,

    /// Optional: algorithm allow-list override. Default: `EdDSA`, `ES256`.
    pub allowed_algorithms: Vec<String>,
}

impl Default for JwksServiceConfig {
    fn default() -> Self {
        Self {
            jwks_base_url: "http://localhost:8105".to_string(),
            issuer: "https://idam.seasame-idam.microscaler.local".to_string(),
            audience: "default.seasame-idam.microscaler.local".to_string(),
            cache_ttl_secs: 300,
            leeway_secs: 60,
            allowed_algorithms: ALLOWED_JWT_ALGORITHMS
                .iter()
                .copied()
                .map(str::to_owned)
                .collect(),
        }
    }
}

impl JwksServiceConfig {
    /// Return the full JWKS URL.
    #[must_use]
    pub fn jwks_url(&self) -> String {
        format!("{}/.well-known/jwks.json", self.jwks_base_url)
    }

    /// Return the full OIDC Discovery URL.
    #[must_use]
    pub fn discovery_url(&self) -> String {
        format!("{}/.well-known/openid-configuration", self.jwks_base_url)
    }
}

// ─── JWKS consumer provider builder ──────────────────────────────────────────

/// Builder for a `JwksBearerProvider` with per-service configuration.
pub struct JwksProviderBuilder {
    config: JwksServiceConfig,
}

impl JwksProviderBuilder {
    /// Create a new builder with the given service configuration.
    #[must_use]
    pub fn new(config: JwksServiceConfig) -> Self {
        Self { config }
    }

    /// Build a `JwksBearerProvider` with the configured settings.
    #[must_use]
    #[allow(clippy::cast_sign_loss)]
    pub fn build(self) -> JwksBearerProvider {
        let mut provider = JwksBearerProvider::new(self.config.jwks_url());

        // Set issuer if non-default.
        if self.config.issuer != "https://idam.seasame-idam.microscaler.local" {
            provider = provider.issuer(&self.config.issuer);
        }

        // Set audience if non-default.
        if self.config.audience != "default.seasame-idam.microscaler.local" {
            provider = provider.audience(&self.config.audience);
        }

        // Set cache TTL (default 300s = 5 min).
        if self.config.cache_ttl_secs != 300 {
            use std::time::Duration;
            provider = provider.cache_ttl(Duration::from_secs(self.config.cache_ttl_secs));
        }

        // Set leeway (default 60s).
        if self.config.leeway_secs != 60 {
            provider = provider.leeway(self.config.leeway_secs as u64);
        }

        provider
    }

    /// Build and return the config alongside the provider (for metadata).
    #[must_use]
    pub fn build_with_config(&self) -> (JwksBearerProvider, JwksServiceConfig) {
        let config = self.config.clone();
        let builder = JwksProviderBuilder {
            config: config.clone(),
        };
        let provider = builder.build();
        (provider, config)
    }
}

// ─── Per-service default configs ─────────────────────────────────────────────

/// Default JWKS config for each service. These match the table in Story 1.3.
///
/// identity-login-service (the token issuer — doesn't validate, but may validate its own tokens).
pub static IDENTITY_LOGIN_CONFIG: std::sync::LazyLock<JwksServiceConfig> =
    std::sync::LazyLock::new(|| JwksServiceConfig {
        jwks_base_url: "http://localhost:8105".to_string(),
        issuer: "https://idam.seasame-idam.microscaler.local".to_string(),
        audience: "identity-login.seasame-idam.microscaler.local".to_string(),
        cache_ttl_secs: 300,
        leeway_secs: 60,
        allowed_algorithms: vec!["EdDSA".to_string()],
    });

/// identity-session-service (serves JWKS, may validate its own tokens).
pub static IDENTITY_SESSION_CONFIG: std::sync::LazyLock<JwksServiceConfig> =
    std::sync::LazyLock::new(|| JwksServiceConfig {
        jwks_base_url: "http://localhost:8105".to_string(),
        issuer: "https://idam.seasame-idam.microscaler.local".to_string(),
        audience: "identity-session.seasame-idam.microscaler.local".to_string(),
        cache_ttl_secs: 300,
        leeway_secs: 60,
        allowed_algorithms: vec!["EdDSA".to_string()],
    });

/// identity-user-mgmt-service.
pub static IDENTITY_USER_MGMT_CONFIG: std::sync::LazyLock<JwksServiceConfig> =
    std::sync::LazyLock::new(|| JwksServiceConfig {
        jwks_base_url: "http://localhost:8105".to_string(),
        issuer: "https://idam.seasame-idam.microscaler.local".to_string(),
        audience: "identity-user-mgmt.seasame-idam.microscaler.local".to_string(),
        cache_ttl_secs: 300,
        leeway_secs: 60,
        allowed_algorithms: vec!["EdDSA".to_string()],
    });

/// authz-core (EXTREME frequency — JWKS cache is critical).
pub static AUTHZ_CORE_CONFIG: std::sync::LazyLock<JwksServiceConfig> =
    std::sync::LazyLock::new(|| JwksServiceConfig {
        jwks_base_url: "http://localhost:8105".to_string(),
        issuer: "https://idam.seasame-idam.microscaler.local".to_string(),
        audience: "authz-core.seasame-idam.microscaler.local".to_string(),
        cache_ttl_secs: 300,
        leeway_secs: 60,
        allowed_algorithms: vec!["EdDSA".to_string()],
    });

/// api-keys.
pub static API_KEYS_CONFIG: std::sync::LazyLock<JwksServiceConfig> =
    std::sync::LazyLock::new(|| JwksServiceConfig {
        jwks_base_url: "http://localhost:8105".to_string(),
        issuer: "https://idam.seasame-idam.microscaler.local".to_string(),
        audience: "api-keys.seasame-idam.microscaler.local".to_string(),
        cache_ttl_secs: 300,
        leeway_secs: 60,
        allowed_algorithms: vec!["EdDSA".to_string()],
    });

/// org-mgmt.
pub static ORG_MGMT_CONFIG: std::sync::LazyLock<JwksServiceConfig> =
    std::sync::LazyLock::new(|| JwksServiceConfig {
        jwks_base_url: "http://localhost:8105".to_string(),
        issuer: "https://idam.seasame-idam.microscaler.local".to_string(),
        audience: "org-mgmt.seasame-idam.microscaler.local".to_string(),
        cache_ttl_secs: 300,
        leeway_secs: 60,
        allowed_algorithms: vec!["EdDSA".to_string()],
    });

// ─── Anti-poisoning guard ────────────────────────────────────────────────────

/// Bucketize a key count into a non-sensitive range string.
///
/// Mitigates HACK-921: exact key counts in span attributes help attackers
/// map the key rotation schedule. Bucketed counts preserve observability
/// without leaking precise rotation state.
#[must_use]
fn keys_count_bucket(count: usize) -> &'static str {
    match count {
        0 => "0",
        1..=2 => "1-2",
        3..=5 => "3-5",
        _ => "6+",
    }
}

/// Validate that a new JWKS set contains at least one key from the previous set.
///
/// This is the critical fix for HACK-101: if the JWKS is poisoned with
/// attacker-controlled keys, the consumer rejects the refresh and retains
/// the old (trusted) key set.
///
/// Returns `true` if the refresh is safe (at least one overlapping key).
///
/// # Security (HACK-921)
///
/// Key counts are bucketized into "1-2", "3-5", "6+" to prevent rotation
/// schedule mapping via span attribute analysis in Jaeger.
#[must_use]
pub fn validate_jwks_refresh(new_keys: &[String], old_keys: &[String]) -> bool {
    let count = new_keys.len();
    let bucket = keys_count_bucket(count);
    let span = tracing::span!(
        tracing::Level::INFO,
        "jwks.cache.refresh",
        keys_count_bucket = bucket
    );
    let _guard = span.enter();

    // If there is no previous set, accept (first fetch).
    if old_keys.is_empty() {
        span.record("cache_status", "miss");
        span.record("result", "allowed");
        tracing::info!("jwks cache miss (first fetch)");
        return true;
    }
    // At least one kid must overlap.
    let ok = new_keys.iter().any(|kid| old_keys.contains(kid));
    if ok {
        span.record("cache_status", "hit");
        span.record("result", "allowed");
        tracing::info!(
            keys_count_bucket = bucket,
            "jwks cache refresh OK (overlap found)"
        );
    } else {
        span.record("cache_status", "miss");
        span.record("result", "denied");
        span.record("error", "no_overlap");
        tracing::warn!("jwks cache refresh REJECTED (no overlap) — possible poisoning");
    }
    ok
}

// ─── Health check ────────────────────────────────────────────────────────────

/// Health check result for the JWKS subsystem.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JwksHealthCheck {
    /// Whether JWKS is reachable.
    pub reachable: bool,
    /// Number of keys in the cached JWKS.
    pub key_count: usize,
    /// Key IDs currently in cache.
    pub key_ids: Vec<String>,
    /// Last successful JWKS fetch timestamp (as epoch seconds).
    pub last_fetch: Option<u64>,
    /// Error message if unreachable.
    pub error: Option<String>,
}

impl JwksHealthCheck {
    /// Build a healthy health check (for when the provider is available).
    #[must_use]
    pub fn healthy(key_count: usize, key_ids: Vec<String>, last_fetch: Option<u64>) -> Self {
        Self {
            reachable: true,
            key_count,
            key_ids,
            last_fetch,
            error: None,
        }
    }

    /// Build an unhealthy health check.
    #[must_use]
    pub fn unreachable(error: String) -> Self {
        Self {
            reachable: false,
            key_count: 0,
            key_ids: Vec::new(),
            last_fetch: None,
            error: Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reject_none_algorithm() {
        assert!(reject_none_algorithm("none"));
        assert!(reject_none_algorithm("None"));
        assert!(reject_none_algorithm("NONE"));
        assert!(!reject_none_algorithm("EdDSA"));
        assert!(!reject_none_algorithm("HS256"));
    }

    #[test]
    fn test_allowed_algorithms() {
        assert!(is_allowed_algorithm("EdDSA"));
        assert!(is_allowed_algorithm("ES256"));
        assert!(!is_allowed_algorithm("HS256"));
        assert!(!is_allowed_algorithm("RS256"));
        assert!(!is_allowed_algorithm("none"));
    }

    #[test]
    fn test_jwks_url_construction() {
        let config = JwksServiceConfig::default();
        assert_eq!(
            config.jwks_url(),
            "http://localhost:8105/.well-known/jwks.json"
        );
    }

    #[test]
    fn test_discovery_url_construction() {
        let config = JwksServiceConfig::default();
        assert_eq!(
            config.discovery_url(),
            "http://localhost:8105/.well-known/openid-configuration"
        );
    }

    #[test]
    fn test_validate_jwks_refresh_no_previous_set() {
        // First fetch: always accept.
        assert!(validate_jwks_refresh(&["key-1".to_string()], &[]));
    }

    #[test]
    fn test_validate_jwks_refresh_with_overlap() {
        let old = vec!["key-1".to_string(), "key-2".to_string()];
        let new = vec!["key-2".to_string(), "key-3".to_string()];
        assert!(validate_jwks_refresh(&new, &old));
    }

    #[test]
    fn test_validate_jwks_refresh_no_overlap_poisoning() {
        let old = vec!["key-1".to_string()];
        let new = vec!["key-forged".to_string()];
        assert!(!validate_jwks_refresh(&new, &old));
    }

    #[test]
    fn test_validation_result_reasons() {
        assert_eq!(ValidationResult::Valid.reason(), "valid");
        assert_eq!(
            ValidationResult::InvalidAlgorithm {
                expected: vec!["EdDSA"],
                got: "HS256".to_string(),
            }
            .reason(),
            "alg"
        );
        assert_eq!(
            ValidationResult::Expired {
                exp: 100,
                now: 200,
                leeway: 60,
            }
            .reason(),
            "exp"
        );
    }

    #[test]
    fn test_healthy_health_check() {
        let hc =
            JwksHealthCheck::healthy(2, vec!["key-1".into(), "key-2".into()], Some(1_700_000_000));
        assert!(hc.reachable);
        assert_eq!(hc.key_count, 2);
        assert!(hc.error.is_none());
    }

    #[test]
    fn test_unhealthy_health_check() {
        let hc = JwksHealthCheck::unreachable("connection refused".to_string());
        assert!(!hc.reachable);
        assert_eq!(hc.key_count, 0);
        assert_eq!(hc.error.unwrap(), "connection refused");
    }
}
