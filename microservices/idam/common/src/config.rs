/// Shared application configuration types loaded from `config/config.yaml`.
///
/// Mirrors the config structure in `gen/main.rs` so both the generated
/// and implementation crates read the same YAML schema.
///
/// All 6 IDAM services use this module. Service-specific extensions
/// (`JwtConfig`, `RedisConfig`) are available as optional fields.
///
/// # Config structure
///
/// ```yaml
/// port: 8080
/// security:
///   jwks:
///     BearerAuth:
///       jwks_url: "http://identity-session-service:8105/.well-known/jwks.json"
///       iss: "https://idam.example.com"
///       aud: "authz-core.myapp.com"
///       leeway_secs: 60
///       cache_ttl_secs: 300
///   api_keys:
///     ApiKeyHeader:
///       key: "test-key"
/// http:
///   keep_alive: true
///   timeout_secs: 30
/// cors:
///   origins: ["https://myapp.com"]
/// ```
///
/// # Design rationale
///
/// Config structs are shared across all 6 services to eliminate
/// duplication. Each service reads from this single source of truth.
use std::collections::HashMap;

/// Top-level application configuration.
///
/// All fields are optional with `Default` — missing sections use sensible
/// defaults. The service starts successfully even without a config file.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct AppConfig {
    /// HTTP port (overrides PORT env var when set).
    pub port: Option<u16>,
    /// Security scheme configurations (JWKS, API keys).
    pub security: Option<SecurityConfig>,
    /// HTTP server tuning options.
    pub http: Option<HttpConfig>,
    /// CORS policy.
    pub cors: Option<CorsConfig>,
    /// JWT token configuration (TTL, refresh token settings).
    pub jwt: Option<JwtConfig>,
    /// Redis connection configuration for push invalidation.
    pub redis: Option<RedisConfig>,
}

/// Security scheme configurations for the service.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct SecurityConfig {
    /// JWKS-based JWT validation configurations, keyed by scheme name.
    pub jwks: Option<HashMap<String, JwksSchemeConfig>>,
    /// API key configurations, keyed by scheme name.
    pub api_keys: Option<HashMap<String, ApiKeyConfig>>,
}

/// JWKS-based JWT validation configuration for a single security scheme.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct JwksSchemeConfig {
    /// URL of the JWKS endpoint (e.g. `http://identity-session-service:8105/.well-known/jwks.json`).
    pub jwks_url: String,
    /// Expected `iss` (issuer) claim. If omitted, no issuer check.
    pub iss: Option<String>,
    /// Expected `aud` (audience) claim. If omitted, no audience check.
    pub aud: Option<String>,
    /// Clock skew tolerance in seconds for `exp`/`nbf` validation.
    pub leeway_secs: Option<u64>,
    /// JWKS cache TTL in seconds.
    pub cache_ttl_secs: Option<u64>,
}

/// API key configuration for a single security scheme.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct ApiKeyConfig {
    /// Expected API key value. In production this should come from vault/KMS.
    pub key: Option<String>,
}

/// HTTP server tuning options.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct HttpConfig {
    /// Enable HTTP keep-alive.
    pub keep_alive: Option<bool>,
    /// Request timeout in seconds.
    pub timeout_secs: Option<u64>,
    /// Maximum concurrent requests.
    pub max_requests: Option<u64>,
}

/// CORS policy.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct CorsConfig {
    /// Allowed origins (e.g. `["https://myapp.com"]`).
    pub origins: Option<Vec<String>>,
    /// Allowed request headers.
    pub allowed_headers: Option<Vec<String>>,
    /// Allowed HTTP methods.
    pub allowed_methods: Option<Vec<String>>,
    /// Whether to include credentials.
    pub allow_credentials: Option<bool>,
    /// Headers exposed to the browser.
    pub expose_headers: Option<Vec<String>>,
    /// Cache duration for preflight responses in seconds.
    pub max_age: Option<u32>,
}

/// Redis connection configuration.
///
/// Used by the push invalidation publisher to connect to Redis for
/// pub/sub event delivery.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct RedisConfig {
    /// Redis connection URL (e.g., `redis://127.0.0.1:6379`).
    pub url: Option<String>,
    /// HMAC-SHA256 secret for signing version bump events.
    pub hmac_secret: Option<String>,
}

/// Load configuration from a YAML file.
///
/// Returns `Ok` with defaults on `NotFound` — the service starts successfully
/// without a config file. Other errors indicate a parse or read failure
/// and should be treated as fatal.
pub fn load_config(path: &std::path::PathBuf) -> Result<AppConfig, String> {
    match std::fs::read_to_string(path) {
        Ok(s) => serde_yaml::from_str::<AppConfig>(&s)
            .map_err(|e| format!("failed to parse {}: {}", path.display(), e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(AppConfig::default()),
        Err(e) => Err(format!("failed to read {}: {}", path.display(), e)),
    }
}

/// JWT token configuration loaded from `config.yaml`.
///
/// Environment variables take priority over config.yaml values.
/// See [`crate::jwt::ttl::TtlConfig::from_env_and_config`] for resolution logic.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct JwtConfig {
    /// Access token TTL configuration.
    pub access_token: Option<AccessTokenConfig>,
}

/// Access token TTL configuration from config.yaml.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct AccessTokenConfig {
    /// Normal user access token TTL in seconds (default: 300 = 5 minutes).
    pub normal_ttl_secs: Option<u64>,
    /// Elevated privilege user access token TTL in seconds (default: 300 = 5 minutes, F-010 aligned).
    pub elevated_ttl_secs: Option<u64>,
    /// Admin user access token TTL in seconds (default: 300 = 5 minutes, F-010 aligned).
    pub admin_ttl_secs: Option<u64>,
    /// Platform user access token TTL in seconds (default: 300 = 5 minutes, F-010 aligned).
    pub platform_ttl_secs: Option<u64>,
    /// Normal user refresh token TTL in days (default: 30).
    pub refresh_ttl_days: Option<u64>,
}
