//! Process-wide [`LifeguardPool`] with [`PooledLifeExecutor`] (lazy init via [`std::sync::OnceLock`]).
//!
//! `DB_HOST` (required), `DB_PORT`, `DB_USER` (default `sesame_idam`), `DB_PASS`, `DB_NAME`, optional `DB_POOL_MAX`
//! (default `2`). TLS: `DB_SSLMODE`, `DB_SSLROOTCERT`, `DB_SSLCERT`, `DB_SSLKEY` (libpq meanings).
//!
//! Outside development the pool refuses to start without `DB_HOST`, and an unset `DB_SSLMODE` means `require` (except
//! to a loopback host). In development (`SESAME_DEV_MODE=1`) the host falls back to the in-cluster Postgres (or
//! `localhost` off-cluster), the password may come from `SESAME_IDAM_DB_PASSWORD`, and an unset `DB_SSLMODE` means
//! libpq's `prefer`.

use std::fmt::Write;
use std::sync::{Arc, OnceLock};

use lifeguard::{query_value, DatabaseConfig, LifeguardPool, PooledLifeExecutor};

mod pre_auth_tenant;
mod rls_context;

pub use pre_auth_tenant::with_pre_auth_tenant;
pub use rls_context::{session_context_from_validated_claims, RlsContextError};

static EXECUTOR: OnceLock<PooledLifeExecutor> = OnceLock::new();

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.trim().is_empty())
}

/// Values safe to print in pod logs (never include password).
pub struct DbSplashMeta {
    pub host: String,
    pub port: String,
    pub database: String,
    pub user: String,
    pub password_configured: bool,
}

fn dev_mode() -> bool {
    std::env::var("SESAME_DEV_MODE")
        .is_ok_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true"))
}

fn load_pool_config() -> (DatabaseConfig, DbSplashMeta) {
    let dev = dev_mode();
    let db_host = non_empty_env("DB_HOST").unwrap_or_else(|| {
        assert!(
            dev,
            "DB_HOST is not set (development only: SESAME_DEV_MODE=1 allows the in-cluster default)"
        );
        std::env::var("KUBERNETES_SERVICE_HOST").map_or_else(
            |_| "localhost".to_string(),
            |_| "postgres.data.svc.cluster.local".to_string(),
        )
    });
    let db_port = std::env::var("DB_PORT").unwrap_or_else(|_| "5432".to_string());
    let db_user = std::env::var("DB_USER").unwrap_or_else(|_| "sesame_idam".to_string());
    let db_pass = non_empty_env("DB_PASS")
        .or_else(|| {
            dev.then(|| non_empty_env("SESAME_IDAM_DB_PASSWORD"))
                .flatten()
        })
        .unwrap_or_default();
    let db_name = std::env::var("DB_NAME").unwrap_or_else(|_| "sesame_idam".to_string());

    let password_configured = !db_pass.trim().is_empty();
    let splash = DbSplashMeta {
        host: db_host.clone(),
        port: db_port.clone(),
        database: db_name.clone(),
        user: db_user.clone(),
        password_configured,
    };

    let mut url = format!("host={db_host} port={db_port} user={db_user} dbname={db_name}");
    if !db_pass.is_empty() {
        let _ = write!(url, " password={db_pass}");
    }
    // TLS, libpq names and meanings: DB_SSLMODE disable | prefer | require | verify-ca | verify-full, DB_SSLROOTCERT,
    // and DB_SSLCERT + DB_SSLKEY for a client certificate. Unset: require, except in development or to a loopback
    // host (a local test database), where libpq's prefer applies.
    let loopback = matches!(db_host.as_str(), "localhost" | "127.0.0.1" | "::1");
    if non_empty_env("DB_SSLMODE").is_none() && !dev && !loopback {
        url.push_str(" sslmode=require");
    }
    for (key, param) in [
        ("DB_SSLMODE", "sslmode"),
        ("DB_SSLROOTCERT", "sslrootcert"),
        ("DB_SSLCERT", "sslcert"),
        ("DB_SSLKEY", "sslkey"),
    ] {
        if let Some(v) = non_empty_env(key) {
            let _ = write!(url, " {param}={v}");
        }
    }

    let cfg = DatabaseConfig {
        url,
        max_connections: std::env::var("DB_POOL_MAX")
            .ok()
            .and_then(|s| s.parse().ok())
            .filter(|&n| n >= 1)
            .unwrap_or(2),
        ..Default::default()
    };
    (cfg, splash)
}

/// Shared pool-backed executor for this process (constructs [`LifeguardPool`] on first use).
///
/// # Panics
///
/// Panics if `LifeguardPool::from_database_config` fails to connect to the database.
#[must_use]
pub fn pooled_executor() -> &'static PooledLifeExecutor {
    EXECUTOR.get_or_init(|| {
        let (cfg, splash) = load_pool_config();

        println!(
            "[startup] Lifeguard: opening database pool → postgresql://{}@{}:{}/{} (password configured: {})",
            splash.user, splash.host, splash.port, splash.database, splash.password_configured
        );

        let pool = LifeguardPool::from_database_config(&cfg, vec![], 0)
            .unwrap_or_else(|e| panic!("LifeguardPool::from_database_config: {e}"));

        println!(
            "[startup] Lifeguard: database pool ready (primary slots={})",
            pool.primary_pool_size()
        );

        let exec = PooledLifeExecutor::new(Arc::new(pool));

        query_value::<i32, _>(&exec, "SELECT 1", &[]).unwrap_or_else(|e| {
            panic!("Lifeguard startup: database connectivity check failed (SELECT 1): {e}");
        });

        println!("[startup] Lifeguard: database connectivity check passed (SELECT 1)");

        exec
    })
}

/// Alias for [`pooled_executor`] (shared pool-backed executor).
#[must_use]
pub fn db() -> &'static PooledLifeExecutor {
    pooled_executor()
}
