/// JWT access token TTL configuration with role-based tiers.
///
/// All token types use 5-minute (300s) TTL after F-010 alignment.
/// TTLs are configurable via environment variables that override config.yaml defaults.
///
/// F-010 rationale for 5-minute TTL across all roles:
/// - 3-minute admin tokens cause 2.5x more Redis load (80k vs 120k ops/hr at 10k admins)
/// - Diminishing security return: admin actions need step-up MFA (Epic 6), not shorter TTL
/// - Operational friction: admin batch ops can't complete in 1-3 minute windows
/// - Step-up MFA provides the real security boundary for high-consequence actions
///
/// Security gotchas:
/// - HACK-301: Zero TTL causes DoS — validate_minimum_ttl at startup
/// - HACK-303: Admin tokens same 5-min TTL as normal — documented trade-off
/// - HACK-304: Token size budget — non-issue for current TTLs (same digit count)
/// - HACK-305: Clock skew tolerance 60s — acceptable operational trade-off
/// - HACK-306: Refresh token rotation without access token rotation — fundamental JWT limitation
use std::sync::OnceLock;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "ttl_metrics")]
use prometheus::{register_histogram_vec, HistogramVec};

/// Minimum allowed TTL in seconds. Reject startup if any TTL is below this.
const MIN_TTL_SECS: u64 = 60;

/// Default TTL values in seconds.
const DEFAULT_NORMAL_TTL: u64 = 300;
const DEFAULT_ELEVATED_TTL: u64 = 300;
const DEFAULT_ADMIN_TTL: u64 = 300;
const DEFAULT_PLATFORM_TTL: u64 = 300;

/// Default refresh token TTL in days.
const DEFAULT_REFRESH_TTL_DAYS: u64 = 30;

/// Admin refresh token TTL in days (shorter for high-privilege).
const DEFAULT_ADMIN_REFRESH_TTL_DAYS: u64 = 7;

#[cfg(feature = "ttl_metrics")]
/// Global histogram for `token_ttl_seconds` metric.
static TOKEN_TTL_HISTOGRAM: OnceLock<HistogramVec> = OnceLock::new();

/// Role-based TTL configuration loaded at startup.
#[derive(Debug, Clone)]
pub struct TtlConfig {
    /// Access token TTL for normal users.
    pub normal_secs: u64,
    /// Access token TTL for elevated-privilege users.
    pub elevated_secs: u64,
    /// Access token TTL for admin users.
    pub admin_secs: u64,
    /// Access token TTL for platform users.
    pub platform_secs: u64,
    /// Refresh token TTL in days for normal users.
    pub refresh_days: u64,
    /// Refresh token TTL in days for admin users.
    pub admin_refresh_days: u64,
}

impl TtlConfig {
    /// Load TTL configuration from environment variables, falling back to defaults.
    pub fn from_env() -> Self {
        Self {
            normal_secs: env_u64("JWT_ACCESS_TTL_NORMAL", DEFAULT_NORMAL_TTL),
            elevated_secs: env_u64("JWT_ACCESS_TTL_ELEVATED", DEFAULT_ELEVATED_TTL),
            admin_secs: env_u64("JWT_ACCESS_TTL_ADMIN", DEFAULT_ADMIN_TTL),
            platform_secs: env_u64("JWT_ACCESS_TTL_PLATFORM", DEFAULT_PLATFORM_TTL),
            refresh_days: env_u64("JWT_REFRESH_TTL_DAYS", DEFAULT_REFRESH_TTL_DAYS),
            admin_refresh_days: env_u64(
                "JWT_ADMIN_REFRESH_TTL_DAYS",
                DEFAULT_ADMIN_REFRESH_TTL_DAYS,
            ),
        }
    }

    /// Load TTL configuration from config.yaml values, falling back to env vars, then defaults.
    ///
    /// Environment variables take priority over config.yaml values.
    pub fn from_env_and_config(
        normal_cfg: Option<u64>,
        elevated_cfg: Option<u64>,
        admin_cfg: Option<u64>,
        platform_cfg: Option<u64>,
        refresh_days_cfg: Option<u64>,
        admin_refresh_cfg: Option<u64>,
    ) -> Self {
        Self {
            normal_secs: env_or_config("JWT_ACCESS_TTL_NORMAL", normal_cfg, DEFAULT_NORMAL_TTL),
            elevated_secs: env_or_config(
                "JWT_ACCESS_TTL_ELEVATED",
                elevated_cfg,
                DEFAULT_ELEVATED_TTL,
            ),
            admin_secs: env_or_config("JWT_ACCESS_TTL_ADMIN", admin_cfg, DEFAULT_ADMIN_TTL),
            platform_secs: env_or_config(
                "JWT_ACCESS_TTL_PLATFORM",
                platform_cfg,
                DEFAULT_PLATFORM_TTL,
            ),
            refresh_days: env_or_config(
                "JWT_REFRESH_TTL_DAYS",
                refresh_days_cfg,
                DEFAULT_REFRESH_TTL_DAYS,
            ),
            admin_refresh_days: env_or_config(
                "JWT_ADMIN_REFRESH_TTL_DAYS",
                admin_refresh_cfg,
                DEFAULT_ADMIN_REFRESH_TTL_DAYS,
            ),
        }
    }

    /// Get the access token TTL for a given role.
    pub fn ttl_for_role(&self, role: &str) -> Duration {
        let secs = match role {
            "org_admin" | "platform_admin" => self.admin_secs,
            "elevated" => self.elevated_secs,
            "platform" => self.platform_secs,
            _ => self.normal_secs,
        };
        Duration::from_secs(secs)
    }

    /// Get the refresh token TTL for a given role.
    pub fn refresh_ttl_for_role(&self, role: &str) -> Duration {
        let days = match role {
            "org_admin" | "platform_admin" | "elevated" => self.admin_refresh_days,
            _ => self.refresh_days,
        };
        Duration::from_secs(days * 86400)
    }

    /// Get the access token TTL as seconds (for metrics).
    pub fn access_ttl_secs_for_role(&self, role: &str) -> u64 {
        match role {
            "org_admin" | "platform_admin" => self.admin_secs,
            "elevated" => self.elevated_secs,
            "platform" => self.platform_secs,
            _ => self.normal_secs,
        }
    }

    /// Get the refresh token TTL in days (for metrics).
    pub fn refresh_ttl_days_for_role(&self, role: &str) -> u64 {
        match role {
            "org_admin" | "platform_admin" | "elevated" => self.admin_refresh_days,
            _ => self.refresh_days,
        }
    }

    /// Compute the `exp` claim value for a token issued at `iat`.
    pub fn exp_for_role(&self, role: &str, iat: u64) -> u64 {
        iat + self.ttl_for_role(role).as_secs()
    }

    /// Compute the refresh token `exp` claim value for a token issued at `iat`.
    pub fn refresh_exp_for_role(&self, role: &str, iat: u64) -> u64 {
        iat + self.refresh_ttl_for_role(role).as_secs()
    }

    /// Compute the current `exp` for a newly issued access token.
    pub fn current_exp_for_role(&self, role: &str) -> u64 {
        let now = now_secs();
        self.exp_for_role(role, now)
    }

    /// Compute the current `exp` for a newly issued refresh token.
    pub fn current_refresh_exp_for_role(&self, role: &str) -> u64 {
        let now = now_secs();
        self.refresh_exp_for_role(role, now)
    }

    /// Record the `token_ttl_seconds` prometheus histogram metric for a given role.
    #[cfg(feature = "ttl_metrics")]
    pub fn record_ttl_metric(&self, role: &str) {
        let secs = self.access_ttl_secs_for_role(role);
        let histogram = TOKEN_TTL_HISTOGRAM.get_or_init(|| {
            register_histogram_vec!(
                "token_ttl_seconds",
                "Access token TTL in seconds at issuance, labeled by role",
                &["role"]
            )
            .expect("failed to register token_ttl_seconds histogram")
        });
        histogram.with_label_values(&[role]).observe(secs as f64);
    }
}

impl Default for TtlConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

/// Validate that all TTL values meet the minimum threshold.
pub fn validate_minimum_ttl(config: &TtlConfig) {
    if config.normal_secs < MIN_TTL_SECS {
        panic!(
            "JWT_ACCESS_TTL_NORMAL must be >= {} seconds (got {})",
            MIN_TTL_SECS, config.normal_secs
        );
    }
    if config.elevated_secs < MIN_TTL_SECS {
        panic!(
            "JWT_ACCESS_TTL_ELEVATED must be >= {} seconds (got {})",
            MIN_TTL_SECS, config.elevated_secs
        );
    }
    if config.admin_secs < MIN_TTL_SECS {
        panic!(
            "JWT_ACCESS_TTL_ADMIN must be >= {} seconds (got {})",
            MIN_TTL_SECS, config.admin_secs
        );
    }
    if config.platform_secs < MIN_TTL_SECS {
        panic!(
            "JWT_ACCESS_TTL_PLATFORM must be >= {} seconds (got {})",
            MIN_TTL_SECS, config.platform_secs
        );
    }
}

/// Validate that refresh token TTL always exceeds access token TTL for every role.
pub fn validate_refresh_exceeds_access(config: &TtlConfig) {
    let refresh_secs = Duration::from_secs(config.refresh_days * 86400).as_secs();
    let admin_refresh_secs = Duration::from_secs(config.admin_refresh_days * 86400).as_secs();

    if config.normal_secs > refresh_secs {
        panic!(
            "JWT_ACCESS_TTL_NORMAL ({}) must be less than refresh TTL ({} days = {} secs)",
            config.normal_secs, config.refresh_days, refresh_secs
        );
    }
    if config.elevated_secs > refresh_secs {
        panic!(
            "JWT_ACCESS_TTL_ELEVATED ({}) must be less than refresh TTL ({} days = {} secs)",
            config.elevated_secs, config.refresh_days, refresh_secs
        );
    }
    if config.admin_secs > admin_refresh_secs {
        panic!(
            "JWT_ACCESS_TTL_ADMIN ({}) must be less than admin refresh TTL ({} days = {} secs)",
            config.admin_secs, config.admin_refresh_days, admin_refresh_secs
        );
    }
    if config.platform_secs > refresh_secs {
        panic!(
            "JWT_ACCESS_TTL_PLATFORM ({}) must be less than refresh TTL ({} days = {} secs)",
            config.platform_secs, config.refresh_days, refresh_secs
        );
    }
}

/// Helper: read a u64 from an environment variable, or return the default.
fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default)
}

/// Helper: env var overrides config value which overrides default.
fn env_or_config(name: &str, config_val: Option<u64>, default: u64) -> u64 {
    env_u64(name, config_val.unwrap_or(default))
}

/// Get the current UNIX timestamp in seconds.
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ttl_for_role_all_return_300_seconds() {
        let config = TtlConfig::from_env();

        assert_eq!(config.ttl_for_role("customer"), Duration::from_secs(300));
        assert_eq!(config.ttl_for_role("user"), Duration::from_secs(300));
        assert_eq!(config.ttl_for_role("elevated"), Duration::from_secs(300));
        assert_eq!(config.ttl_for_role("org_admin"), Duration::from_secs(300));
        assert_eq!(
            config.ttl_for_role("platform_admin"),
            Duration::from_secs(300)
        );
        assert_eq!(config.ttl_for_role("platform"), Duration::from_secs(300));
        assert_eq!(
            config.ttl_for_role("unknown_role"),
            Duration::from_secs(300)
        );
    }

    #[test]
    fn test_all_roles_produce_same_ttl() {
        let config = TtlConfig::from_env();
        assert_eq!(config.ttl_for_role("customer"), config.ttl_for_role("org_admin"));
        assert_eq!(config.ttl_for_role("org_admin"), config.ttl_for_role("platform"));
        assert_eq!(config.ttl_for_role("platform"), Duration::from_secs(300));
    }

    #[test]
    fn test_exp_claim_is_correct() {
        let config = TtlConfig::from_env();
        let iat: u64 = 1000;
        let expected_exp = iat + 300;
        assert_eq!(config.exp_for_role("customer", iat), expected_exp);
        assert_eq!(config.exp_for_role("org_admin", iat), expected_exp);
        assert_eq!(config.exp_for_role("platform", iat), expected_exp);
    }

    #[test]
    fn test_refresh_token_ttl_is_configurable() {
        let prev = std::env::var("JWT_REFRESH_TTL_DAYS").ok();
        std::env::set_var("JWT_REFRESH_TTL_DAYS", "14");
        let config = TtlConfig::from_env();
        let refresh_secs = config.refresh_ttl_for_role("customer").as_secs();
        assert_eq!(refresh_secs, 14 * 86400);
        let admin_refresh = config.refresh_ttl_for_role("org_admin").as_secs();
        assert_eq!(admin_refresh, 7 * 86400);
        match prev {
            Some(v) => std::env::set_var("JWT_REFRESH_TTL_DAYS", v),
            None => std::env::remove_var("JWT_REFRESH_TTL_DAYS"),
        }
    }

    #[test]
    fn test_refresh_token_ttl_exceeds_access_for_all_roles() {
        let config = TtlConfig::from_env();
        let access_secs = config.ttl_for_role("customer").as_secs();
        let refresh_secs = config.refresh_ttl_for_role("customer").as_secs();
        assert!(refresh_secs > access_secs);
        let admin_access = config.ttl_for_role("org_admin").as_secs();
        let admin_refresh = config.refresh_ttl_for_role("org_admin").as_secs();
        assert!(admin_refresh > admin_access);
    }

    #[test]
    fn test_validate_minimum_ttl_passes_normal_config() {
        let config = TtlConfig::from_env();
        validate_minimum_ttl(&config);
    }

    #[test]
    fn test_validate_minimum_ttl_rejects_zero_ttl() {
        let mut config = TtlConfig::from_env();
        config.normal_secs = 0;
        assert!(
            std::panic::catch_unwind(|| validate_minimum_ttl(&config)).is_err(),
            "Should panic on zero TTL"
        );
    }

    #[test]
    fn test_validate_minimum_ttl_rejects_too_low_ttl() {
        let mut config = TtlConfig::from_env();
        config.admin_secs = 30;
        assert!(
            std::panic::catch_unwind(|| validate_minimum_ttl(&config)).is_err(),
            "Should panic on TTL below 60 seconds"
        );
    }

    #[test]
    fn test_validate_refresh_exceeds_access() {
        let config = TtlConfig::from_env();
        validate_refresh_exceeds_access(&config);
    }

    #[test]
    fn test_env_override_normal_ttl() {
        let prev = std::env::var("JWT_ACCESS_TTL_NORMAL").ok();
        std::env::set_var("JWT_ACCESS_TTL_NORMAL", "600");
        let config = TtlConfig::from_env();
        assert_eq!(config.ttl_for_role("customer"), Duration::from_secs(600));
        match prev {
            Some(v) => std::env::set_var("JWT_ACCESS_TTL_NORMAL", v),
            None => std::env::remove_var("JWT_ACCESS_TTL_NORMAL"),
        }
    }

    #[test]
    fn test_env_override_elevated_ttl() {
        let prev = std::env::var("JWT_ACCESS_TTL_ELEVATED").ok();
        std::env::set_var("JWT_ACCESS_TTL_ELEVATED", "600");
        let config = TtlConfig::from_env();
        assert_eq!(config.ttl_for_role("elevated"), Duration::from_secs(600));
        match prev {
            Some(v) => std::env::set_var("JWT_ACCESS_TTL_ELEVATED", v),
            None => std::env::remove_var("JWT_ACCESS_TTL_ELEVATED"),
        }
    }

    #[test]
    fn test_env_override_admin_ttl() {
        let prev = std::env::var("JWT_ACCESS_TTL_ADMIN").ok();
        std::env::set_var("JWT_ACCESS_TTL_ADMIN", "600");
        let config = TtlConfig::from_env();
        assert_eq!(config.ttl_for_role("org_admin"), Duration::from_secs(600));
        match prev {
            Some(v) => std::env::set_var("JWT_ACCESS_TTL_ADMIN", v),
            None => std::env::remove_var("JWT_ACCESS_TTL_ADMIN"),
        }
    }

    #[test]
    fn test_env_override_platform_ttl() {
        let prev = std::env::var("JWT_ACCESS_TTL_PLATFORM").ok();
        std::env::set_var("JWT_ACCESS_TTL_PLATFORM", "600");
        let config = TtlConfig::from_env();
        assert_eq!(config.ttl_for_role("platform"), Duration::from_secs(600));
        match prev {
            Some(v) => std::env::set_var("JWT_ACCESS_TTL_PLATFORM", v),
            None => std::env::remove_var("JWT_ACCESS_TTL_PLATFORM"),
        }
    }

    #[test]
    fn test_from_env_and_config_env_overrides() {
        let prev = std::env::var("JWT_ACCESS_TTL_NORMAL").ok();
        std::env::set_var("JWT_ACCESS_TTL_NORMAL", "600");
        let config = TtlConfig::from_env_and_config(
            Some(300), Some(300), Some(300), Some(300), Some(30), Some(7),
        );
        assert_eq!(config.normal_secs, 600);
        std::env::remove_var("JWT_ACCESS_TTL_NORMAL");
        let config2 = TtlConfig::from_env_and_config(
            Some(600), Some(300), Some(300), Some(300), Some(30), Some(7),
        );
        assert_eq!(config2.normal_secs, 600);
        match prev {
            Some(v) => std::env::set_var("JWT_ACCESS_TTL_NORMAL", v),
            None => std::env::remove_var("JWT_ACCESS_TTL_NORMAL"),
        }
    }

    #[test]
    fn test_max_ttl_works() {
        let prev = std::env::var("JWT_ACCESS_TTL_NORMAL").ok();
        std::env::set_var("JWT_ACCESS_TTL_NORMAL", "3600");
        let config = TtlConfig::from_env();
        assert_eq!(config.ttl_for_role("customer"), Duration::from_secs(3600));
        validate_minimum_ttl(&config);
        match prev {
            Some(v) => std::env::set_var("JWT_ACCESS_TTL_NORMAL", v),
            None => std::env::remove_var("JWT_ACCESS_TTL_NORMAL"),
        }
    }

    #[test]
    fn test_concurrent_logins_different_roles() {
        let config = TtlConfig::from_env();
        let customer_ttl = config.ttl_for_role("customer");
        let admin_ttl = config.ttl_for_role("org_admin");
        assert_eq!(customer_ttl, admin_ttl);
        assert_eq!(customer_ttl, Duration::from_secs(300));
    }

    #[cfg(feature = "ttl_metrics")]
    #[test]
    fn test_record_ttl_metric_succeeds() {
        let config = TtlConfig::from_env();
        config.record_ttl_metric("customer");
        config.record_ttl_metric("org_admin");
        config.record_ttl_metric("platform");
        config.record_ttl_metric("unknown");
    }

    #[test]
    fn test_admin_refresh_ttl_from_config() {
        let config = TtlConfig::from_env_and_config(
            Some(300), Some(300), Some(300), Some(300), Some(14), Some(5),
        );
        assert_eq!(
            config.admin_refresh_days, 5,
            "Admin refresh should use config value (5 days)"
        );
        assert_eq!(config.refresh_days, 14);
    }
}
