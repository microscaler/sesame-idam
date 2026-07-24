//! Gate A3 acceptance: OTP send abuse & toll-fraud controls
//! (`TASKS-staging-hardening.md` A3).
//!
//! - repeated sends to one recipient cap out (window + daily)
//! - rapid identical re-sends dedupe
//! - SMS requires tenant opt-in (ADR-008 interim env allow-list)
//! - the global daily SMS spend cannot exceed the configured ceiling
//! - the HTTP surface returns the SAME generic success whether a send was
//!   dispatched or suppressed (no cap oracle, no enumeration)
//!
//! Guard-level tests need Redis only; the controller test also needs
//! Postgres (tenant gate). Each nextest test runs in its own process, so
//! env policy overrides are isolated.

use std::net::TcpStream;
use std::sync::Once;
use std::time::Duration;

use brrtrouter::typed::TypedHandlerRequest;
use http::Method;

use sesame_idam_identity_login_service::controllers::login_email_otp;
use sesame_idam_identity_login_service::services::abuse_guard::{self, Channel, SendDecision};
use sesame_idam_identity_login_service_gen::handlers::login_email_otp::Request as EmailOtpRequest;

use crate::common::ensure_active_tenant;

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

fn unique_tenant(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::new_v4())
}

/// Scenario: sends to one recipient cap out within the window.
#[test]
fn recipient_window_cap_enforced() {
    if !redis_available() {
        println!("SKIP: Redis not available");
        return;
    }
    std::env::set_var("OTP_SEND_DEDUPE_SECS", "0");
    std::env::set_var("OTP_SEND_MAX_PER_WINDOW", "3");
    std::env::set_var("OTP_SEND_MAX_PER_DAY", "100");

    let tenant = unique_tenant("wincap");
    let recipient = "flood-me@example.com";
    for i in 1..=3 {
        assert_eq!(
            abuse_guard::gate_otp_send(&tenant, Channel::Email, recipient),
            SendDecision::Allow,
            "send {i} should be allowed"
        );
    }
    assert_eq!(
        abuse_guard::gate_otp_send(&tenant, Channel::Email, recipient),
        SendDecision::Capped,
        "4th send in the window must cap out"
    );
}

/// Scenario: rapid identical re-sends are deduped without burning quota.
#[test]
fn rapid_resend_deduped() {
    if !redis_available() {
        println!("SKIP: Redis not available");
        return;
    }
    std::env::set_var("OTP_SEND_DEDUPE_SECS", "60");

    let tenant = unique_tenant("dedupe");
    let recipient = "impatient@example.com";
    assert_eq!(
        abuse_guard::gate_otp_send(&tenant, Channel::Email, recipient),
        SendDecision::Allow
    );
    assert_eq!(
        abuse_guard::gate_otp_send(&tenant, Channel::Email, recipient),
        SendDecision::Deduped,
        "immediate identical re-send must dedupe"
    );
}

/// Scenario: the daily per-recipient ceiling holds even when the short
/// window would allow more.
#[test]
fn recipient_daily_cap_enforced() {
    if !redis_available() {
        println!("SKIP: Redis not available");
        return;
    }
    std::env::set_var("OTP_SEND_DEDUPE_SECS", "0");
    std::env::set_var("OTP_SEND_MAX_PER_WINDOW", "100");
    std::env::set_var("OTP_SEND_MAX_PER_DAY", "2");

    let tenant = unique_tenant("daycap");
    let recipient = "daily@example.com";
    assert_eq!(
        abuse_guard::gate_otp_send(&tenant, Channel::Email, recipient),
        SendDecision::Allow
    );
    assert_eq!(
        abuse_guard::gate_otp_send(&tenant, Channel::Email, recipient),
        SendDecision::Allow
    );
    assert_eq!(
        abuse_guard::gate_otp_send(&tenant, Channel::Email, recipient),
        SendDecision::Capped,
        "3rd send of the day must cap out"
    );
}

/// Scenario: SMS is refused for tenants that have not opted in (ADR-008),
/// and allowed once the tenant is on the allow-list.
#[test]
fn sms_requires_tenant_opt_in() {
    if !redis_available() {
        println!("SKIP: Redis not available");
        return;
    }
    std::env::set_var("OTP_SEND_DEDUPE_SECS", "0");
    std::env::set_var("SMS_SPEND_SCOPE", &unique_tenant("optin-scope"));

    let tenant = unique_tenant("smsoptin");
    std::env::set_var("SMS_OPTED_IN_TENANTS", "some-other-tenant");
    assert_eq!(
        abuse_guard::gate_otp_send(&tenant, Channel::Sms, "+15551230001"),
        SendDecision::TenantNotOptedIn
    );

    std::env::set_var("SMS_OPTED_IN_TENANTS", format!("some-other-tenant,{tenant}"));
    assert_eq!(
        abuse_guard::gate_otp_send(&tenant, Channel::Sms, "+15551230001"),
        SendDecision::Allow
    );
}

/// Scenario: the GLOBAL daily SMS spend ceiling bounds toll fraud even when
/// the attacker rotates recipient numbers.
#[test]
fn sms_global_spend_ceiling_enforced() {
    if !redis_available() {
        println!("SKIP: Redis not available");
        return;
    }
    let tenant = unique_tenant("smsbudget");
    std::env::set_var("OTP_SEND_DEDUPE_SECS", "0");
    std::env::set_var("SMS_OPTED_IN_TENANTS", &tenant);
    std::env::set_var("SMS_DAILY_SPEND_CEILING_CENTS", "10");
    std::env::set_var("SMS_COST_CENTS", "5");
    std::env::set_var("SMS_SPEND_SCOPE", &unique_tenant("budget-scope"));

    // Distinct recipients defeat per-recipient caps; the budget still holds.
    assert_eq!(
        abuse_guard::gate_otp_send(&tenant, Channel::Sms, "+15551230001"),
        SendDecision::Allow
    );
    assert_eq!(
        abuse_guard::gate_otp_send(&tenant, Channel::Sms, "+15551230002"),
        SendDecision::Allow
    );
    assert_eq!(
        abuse_guard::gate_otp_send(&tenant, Channel::Sms, "+15551230003"),
        SendDecision::BudgetExhausted,
        "spend past the ceiling must be blocked"
    );
}

/// Scenario: the HTTP surface leaks nothing — an allowed send and a capped
/// send return byte-identical generic success bodies.
#[test]
fn capped_send_response_indistinguishable() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    if !redis_available() {
        println!("SKIP: Redis not available");
        return;
    }
    std::env::set_var("OTP_SEND_DEDUPE_SECS", "0");
    std::env::set_var("OTP_SEND_MAX_PER_WINDOW", "1");

    let tenant = "otpcaps-bdd-tenant";
    ensure_active_tenant(tenant);

    let request = |email: &str| TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/login/email-otp".to_string(),
        handler_name: "login_email_otp".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: EmailOtpRequest {
            email: email.to_string(),
            x_tenant_id: tenant.to_string(),
        },
        jwt_claims: None,
    };

    let email = format!("otp_{}@example.com", uuid::Uuid::new_v4());
    let first = login_email_otp::handle(request(&email)); // allowed
    let second = login_email_otp::handle(request(&email)); // capped (window=1)

    assert_eq!(first.status, 200);
    assert_eq!(second.status, 200);
    assert_eq!(
        first.body, second.body,
        "allowed and suppressed sends must be indistinguishable to the caller"
    );
    assert_eq!(first.body["success"], true);
}
