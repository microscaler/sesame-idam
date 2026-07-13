// Minimal test database setup for org-mgmt.
// Uses environment variables: TEST_DB_HOST, TEST_DB_PORT, TEST_DB_USER, TEST_DB_PASS, TEST_DB_NAME

use std::net::TcpStream;
use std::sync::Once;
use std::time::Duration;

static INIT: Once = Once::new();

fn postgres_reachable() -> bool {
    let host = std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("TEST_DB_PORT").unwrap_or_else(|_| "5432".to_string());
    format!("{host}:{port}")
        .parse()
        .ok()
        .and_then(|sa| TcpStream::connect_timeout(&sa, Duration::from_millis(500)).ok())
        .is_some()
}

/// Postgres-only infra gate for org-mgmt service tests. Configures the process-wide
/// DB pool env before the first `sesame_idam_database::db()` call. Returns false when
/// Postgres is unreachable so tests SKIP rather than fail in a DB-less environment.
/// org-mgmt lifecycle has no Redis dependency, so Redis is not required here.
pub fn infra_available() -> bool {
    if !postgres_reachable() {
        return false;
    }

    INIT.call_once(|| {
        let host = std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = std::env::var("TEST_DB_PORT").unwrap_or_else(|_| "5432".to_string());

        std::env::set_var("DB_POOL_MAX", "2");
        std::env::set_var("DB_HOST", &host);
        std::env::set_var("DB_PORT", &port);
        std::env::set_var(
            "DB_USER",
            std::env::var("TEST_DB_USER").unwrap_or_else(|_| "sesame_idam".to_string()),
        );
        std::env::set_var(
            "DB_PASS",
            std::env::var("TEST_DB_PASS")
                .unwrap_or_else(|_| "dev_password_change_in_prod".to_string()),
        );
        std::env::set_var(
            "DB_NAME",
            std::env::var("TEST_DB_NAME").unwrap_or_else(|_| "sesame_idam".to_string()),
        );
    });
    true
}

/// A tenant id unique per test run to keep seeded rows isolated.
#[must_use]
pub fn unique_tenant(prefix: &str) -> String {
    format!("bdd-{}-{}", prefix, uuid::Uuid::new_v4())
}
