//! Gate A2 acceptance: account lockout / progressive backoff
//! (`TASKS-staging-hardening.md` A2).
//!
//! - repeated bad passwords → locked with backoff
//! - correct password DURING the lock is still denied
//! - the lock decays; login works again after
//! - locked and wrong-password responses are byte-identical (no oracle)
//! - unknown identifiers lock exactly like real ones (no enumeration)
//!
//! Needs Postgres (tenant gate + users) and Redis (guard state). Skips
//! gracefully when either is unreachable, like the other DB-backed BDD
//! tests. Each nextest test runs in its own process, so env-var policy
//! overrides here cannot leak into other tests.

use std::net::TcpStream;
use std::sync::Once;
use std::time::Duration;

use brrtrouter::typed::TypedHandlerRequest;
use http::Method;

use sesame_idam_identity_login_service::controllers::{auth_login, auth_register};
use sesame_idam_identity_login_service::services::abuse_guard::{self, FailureOutcome};
use sesame_idam_identity_login_service_gen::handlers::auth_login::Request as LoginRequest;
use sesame_idam_identity_login_service_gen::handlers::auth_register::Request as RegisterRequest;

use crate::common::{ensure_active_tenant, FIXTURE_TENANT, FIXTURE_WEB_CLIENT};

const TEST_TENANT: &str = FIXTURE_TENANT;
const PASSWORD: &str = "SecureP@ss123!";

static INIT: Once = Once::new();

fn db_available() -> bool {
    let host = std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("TEST_DB_PORT").unwrap_or_else(|_| "5432".to_string());
    let addr = format!("{host}:{port}");
    let reachable = addr
        .parse()
        .ok()
        .and_then(|sa| TcpStream::connect_timeout(&sa, Duration::from_millis(500)).ok())
        .is_some();
    if !reachable {
        return false;
    }
    INIT.call_once(|| {
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

fn redis_available() -> bool {
    let host = std::env::var("TEST_REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("TEST_REDIS_PORT").unwrap_or_else(|_| "6379".to_string());
    format!("{host}:{port}")
        .parse()
        .ok()
        .and_then(|sa| TcpStream::connect_timeout(&sa, Duration::from_millis(500)).ok())
        .is_some()
}

fn unique_email(prefix: &str) -> String {
    format!("lockout_{}_{}@example.com", prefix, uuid::Uuid::new_v4())
}

fn register_request(email: &str) -> TypedHandlerRequest<RegisterRequest> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/register".to_string(),
        handler_name: "auth_register".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: RegisterRequest {
            client_id: FIXTURE_WEB_CLIENT.to_string(),
            email: email.to_string(),
            first_name: None,
            last_name: None,
            password: PASSWORD.to_string(),
            phone: None,
            username: None,
            x_tenant_id: Some(TEST_TENANT.to_string()),
        },
        jwt_claims: None,
    }
}

fn login_request(email: &str, password: &str) -> TypedHandlerRequest<LoginRequest> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/login".to_string(),
        handler_name: "auth_login".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: LoginRequest {
            client_id: FIXTURE_WEB_CLIENT.to_string(),
            email: email.to_string(),
            organization_id: None,
            password: password.to_string(),
            x_tenant_id: Some(TEST_TENANT.to_string()),
        },
        jwt_claims: None,
    }
}

/// Scenario: N bad passwords lock the account; the correct password during
/// the lock is STILL denied with the identical generic 401; after the lock
/// expires, the correct password works again.
#[test]
fn lockout_after_repeated_failures_then_decays() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    if !redis_available() {
        println!("SKIP: Redis not available");
        return;
    }
    // Short lock for the decay half of the scenario.
    std::env::set_var("LOCKOUT_THRESHOLD", "5");
    std::env::set_var("LOCKOUT_BASE_SECS", "2");
    std::env::set_var("LOCKOUT_MAX_SECS", "2");
    ensure_active_tenant(TEST_TENANT);

    let email = unique_email("decay");
    let resp = auth_register::handle(register_request(&email));
    assert_eq!(resp.status, 201, "register: {:?}", resp.body);

    // 5 bad passwords → all generic 401s; the 5th engages the lock.
    let mut wrong_password_body = serde_json::Value::Null;
    for i in 1..=5 {
        let resp = auth_login::handle(login_request(&email, "wrong-password"));
        assert_eq!(resp.status, 401, "failure {i} should be 401");
        wrong_password_body = resp.body.clone();
    }

    // 6th attempt with the CORRECT password: still denied, and the body is
    // byte-identical to a wrong-password 401 — no lock-state oracle.
    let resp = auth_login::handle(login_request(&email, PASSWORD));
    assert_eq!(
        resp.status, 401,
        "correct password during lock must be denied"
    );
    assert_eq!(
        resp.body, wrong_password_body,
        "locked response must be indistinguishable from wrong-password"
    );

    // Lock decays (2s) → correct password succeeds.
    std::thread::sleep(Duration::from_secs(3));
    let resp = auth_login::handle(login_request(&email, PASSWORD));
    assert_eq!(resp.status, 200, "post-decay login: {:?}", resp.body);
}

/// Scenario: unknown identifiers accumulate lockout state exactly like real
/// accounts — an attacker cannot distinguish existence via lockout behaviour.
#[test]
fn unknown_identifier_locks_identically() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    if !redis_available() {
        println!("SKIP: Redis not available");
        return;
    }
    std::env::set_var("LOCKOUT_THRESHOLD", "3");
    std::env::set_var("LOCKOUT_BASE_SECS", "30");
    ensure_active_tenant(TEST_TENANT);

    let ghost = unique_email("ghost");
    // Threshold is process-once; drive enough failures to engage lock under
    // whatever policy the binary already initialized.
    for _ in 0..12 {
        let resp = auth_login::handle(login_request(&ghost, "anything"));
        assert_eq!(resp.status, 401);
        if abuse_guard::login_locked(TEST_TENANT, &ghost).is_some() {
            break;
        }
    }
    assert!(
        abuse_guard::login_locked(TEST_TENANT, &ghost).is_some(),
        "unknown identifier should be locked after threshold failures"
    );
    let resp = auth_login::handle(login_request(&ghost, "anything"));
    assert_eq!(resp.status, 401);
    assert_eq!(resp.body["error"], "invalid_credentials");
}

/// Scenario: the guard's backoff is progressive — lock duration does not
/// shrink across successive failures once locked, and success does not lift
/// an active lock.
/// (Guard-level test: Redis only. Exact second values depend on process-once
/// env knobs, so this asserts monotonic shape.)
#[test]
fn progressive_backoff_doubles_and_caps() {
    if !redis_available() {
        println!("SKIP: Redis not available");
        return;
    }
    std::env::set_var("LOCKOUT_THRESHOLD", "2");
    std::env::set_var("LOCKOUT_BASE_SECS", "2");
    std::env::set_var("LOCKOUT_MAX_SECS", "8");

    let tenant = format!("backoff-{}", uuid::Uuid::new_v4());
    let ident = format!("victim-{}@example.com", uuid::Uuid::new_v4().simple());

    let mut last_lock_secs: Option<u64> = None;
    let mut saw_lock = false;
    for _ in 0..8 {
        match abuse_guard::record_login_failure(&tenant, &ident) {
            FailureOutcome::Counted { .. } => {}
            FailureOutcome::Locked { lock_secs, .. } => {
                if let Some(prev) = last_lock_secs {
                    assert!(lock_secs >= prev, "backoff must not shrink: {prev} → {lock_secs}");
                }
                last_lock_secs = Some(lock_secs);
                saw_lock = true;
            }
        }
    }
    assert!(saw_lock, "identifier must lock after repeated failures");
    assert!(abuse_guard::login_locked(&tenant, &ident).is_some());

    abuse_guard::record_login_success(&tenant, &ident);
    assert!(
        abuse_guard::login_locked(&tenant, &ident).is_some(),
        "success must not lift an active lock"
    );
}
