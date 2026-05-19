/// Application configuration types loaded from `config/config.yaml`.
///
/// Mirrors the config structure in `gen/main.rs` so both the generated
/// and implementation crates read the same YAML schema.
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
///       aud: "api-keys.myapp.com"
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
    /// URL of the JWKS endpoint.
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

/// Load configuration from a YAML file.
///
/// # Errors
///
/// Returns `Ok` with defaults on `NotFound`. Other errors indicate
/// a parse or read failure and should be treated as fatal.
pub fn load_config(path: &std::path::PathBuf) -> Result<AppConfig, String> {
    match std::fs::read_to_string(path) {
        Ok(s) => serde_yaml::from_str::<AppConfig>(&s)
            .map_err(|e| format!("failed to parse {}: {}", path.display(), e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(AppConfig::default()),
        Err(e) => Err(format!("failed to read {}: {}", path.display(), e)),
    }
}
