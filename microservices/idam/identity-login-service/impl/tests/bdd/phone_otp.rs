//! Phone OTP round trip via the mock SMS outbox + the cost policy.
//!
//! Mirrors the email round trip but reads the code back from the Redis
//! mock-outbox (SMS_PROVIDER=mock) — the Twilio-free e2e seam. Also asserts
//! the cost policy: per-login SMS is OFF by default (no code minted, no
//! outbox entry), and only opens when SMS_ALLOWED_PURPOSES includes login.
//!
//! Needs Postgres (users/tenant) + Redis (otp + outbox). Skips gracefully
//! when either is unreachable. Registration seeds a user, then we set the
//! phone directly on the row (register() only takes email/password).

use std::net::TcpStream;
use std::sync::Once;
use std::time::Duration;

use brrtrouter::typed::TypedHandlerRequest;
use http::Method;

use sesame_idam_identity_login_service::controllers::{login_phone_otp, verify_phone_otp};
use sesame_idam_identity_login_service::services::{otp, sms, user_service::UserService};
use sesame_idam_identity_login_service_gen::handlers::login_phone_otp::Request as SendReq;
use sesame_idam_identity_login_service_gen::handlers::verify_phone_otp::Request as VerifyReq;

use crate::common::ensure_active_tenant;

const TENANT: &str = "phone-otp-bdd-tenant";
static INIT: Once = Once::new();

fn db_available() -> bool {
    let host = std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("TEST_DB_PORT").unwrap_or_else(|_| "5432".to_string());
    let reachable = format!("{host}:{port}")
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
            std::env::var("TEST_DB_PASS").unwrap_or_else(|_| "dev_password_change_in_prod".to_string()),
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

fn unique_phone() -> String {
    // E.164-ish synthetic number, unique per test.
    let n = uuid::Uuid::new_v4().as_u128() % 1_000_000_000;
    format!("+1999{n:09}")
}

/// Seed an active user with a phone directly (register() is email/password).
fn seed_phone_user(phone: &str) -> String {
    let exec = sesame_idam_database::db();
    let email = format!("phoneuser_{}@example.com", uuid::Uuid::new_v4().simple());
    sesame_idam_database::with_pre_auth_tenant(TENANT, |exec| {
        UserService::create_user(TENANT, &email, "x", Some(phone.to_string()), exec)
    })
    .expect("seed user")
    .to_string()
}

fn send_request(phone: &str) -> brrtrouter::typed::HttpJson<serde_json::Value> {
    login_phone_otp::handle(TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/login/phone-otp".to_string(),
        handler_name: "login_phone_otp".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: SendReq { phone: phone.to_string(), x_tenant_id: TENANT.to_string() },
        jwt_claims: None,
    })
}

fn verify_request(phone: &str, code: &str) -> brrtrouter::typed::HttpJson<serde_json::Value> {
    verify_phone_otp::handle(TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/verify/phone-otp".to_string(),
        handler_name: "verify_phone_otp".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: VerifyReq {
            code: code.to_string(),
            phone: phone.to_string(),
            x_tenant_id: TENANT.to_string(),
        },
        jwt_claims: None,
    })
}

/// Cost policy: with default SMS_ALLOWED_PURPOSES (no login), a phone-OTP
/// login request returns generic success but mints NOTHING and sends
/// NOTHING — no outbox entry, and verify fails.
#[test]
fn login_sms_disabled_by_default_no_spend() {
    if !db_available() || !redis_available() {
        println!("SKIP: Postgres/Redis not available");
        return;
    }
    std::env::set_var("SMS_PROVIDER", "mock");
    std::env::remove_var("SMS_ALLOWED_PURPOSES"); // defaults: registration,password_reset
    std::env::set_var("SMS_OPTED_IN_TENANTS", TENANT);
    std::env::set_var("OTP_SEND_DEDUPE_SECS", "0");
    ensure_active_tenant(TENANT);

    let phone = unique_phone();
    seed_phone_user(&phone);

    let resp = send_request(&phone);
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body["success"], true);
    assert!(
        sms::mock_outbox_latest(&phone).is_none(),
        "default policy must not send login SMS"
    );
    // And no verifiable code exists.
    assert!(otp::verify_phone_otp(TENANT, &phone, "000000").is_none());
}

/// With login enabled in policy, the full round trip works: send → mock
/// outbox has the code → verify → real signed JWT; replay rejected.
#[test]
fn phone_otp_round_trip_when_login_enabled() {
    if !db_available() || !redis_available() {
        println!("SKIP: Postgres/Redis not available");
        return;
    }
    std::env::set_var("SMS_PROVIDER", "mock");
    std::env::set_var("SMS_ALLOWED_PURPOSES", "registration,password_reset,login");
    std::env::set_var("SMS_OPTED_IN_TENANTS", TENANT);
    std::env::set_var("OTP_SEND_DEDUPE_SECS", "0");
    ensure_active_tenant(TENANT);

    let phone = unique_phone();
    seed_phone_user(&phone);

    let resp = send_request(&phone);
    assert_eq!(resp.status, 200);

    let msg = sms::mock_outbox_latest(&phone).expect("mock outbox must hold the SMS");
    // Real SMS bodies punctuate around the code (e.g. "is 123456."), so
    // strip non-digits per token before matching the 6-digit run.
    let code: String = msg
        .split_whitespace()
        .map(|w| w.chars().filter(char::is_ascii_digit).collect::<String>())
        .find(|w| w.len() == 6)
        .expect("SMS must contain a 6-digit code")
        .to_string();

    // Wrong code first (skip the 1-in-a-million collision).
    if code != "000000" {
        let wrong = verify_request(&phone, "000000");
        assert_eq!(wrong.status, 401);
    }

    let resp = verify_request(&phone, &code);
    assert_eq!(resp.status, 200, "verify: {:?}", resp.body);
    assert_eq!(resp.body["token_type"], "Bearer");
    let payload = decode_jwt_payload(resp.body["access_token"].as_str().unwrap());
    assert_eq!(payload["tenant_id"], TENANT);

    // Single use.
    let replay = verify_request(&phone, &code);
    assert_eq!(replay.status, 401, "phone OTP must be single-use");
}

/// SMS tenant opt-in (ADR-008) still applies even with login enabled: a
/// tenant not on the allow-list gets no SMS.
#[test]
fn sms_tenant_opt_in_still_required() {
    if !db_available() || !redis_available() {
        println!("SKIP: Postgres/Redis not available");
        return;
    }
    std::env::set_var("SMS_PROVIDER", "mock");
    std::env::set_var("SMS_ALLOWED_PURPOSES", "registration,password_reset,login");
    std::env::set_var("SMS_OPTED_IN_TENANTS", "some-other-tenant");
    std::env::set_var("OTP_SEND_DEDUPE_SECS", "0");
    ensure_active_tenant(TENANT);

    let phone = unique_phone();
    seed_phone_user(&phone);

    let resp = send_request(&phone);
    assert_eq!(resp.status, 200);
    assert!(
        sms::mock_outbox_latest(&phone).is_none(),
        "tenant not opted into SMS must receive nothing"
    );
}

fn decode_jwt_payload(token: &str) -> serde_json::Value {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3, "compact JWT");
    serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[1]).unwrap()).unwrap()
}
