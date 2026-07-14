//! Logout revocation BDD (D3 hardening): logout records the access-token `jti`
//! in the Redis denylist (`denylist:{jti}`) so denylist-aware validation rejects
//! the logged-out access token until it would have expired.
//!
//! ```bash
//! ssh ms02 'source ~/.cargo/env && cd ~/Workspace/microscaler/seasame-idam/microservices && \
//!   cargo test -p sesame_idam_identity_login_service --test main_bdd logout_revocation -- --nocapture'
//! ```

use http::Method;

use brrtrouter::typed::TypedHandlerRequest;
use sesame_idam_identity_login_service::controllers::auth_logout;
use sesame_idam_identity_login_service_gen::handlers::auth_logout::Request as LogoutRequest;

use super::token_lifecycle::infra_available;

const TEST_TENANT: &str = "bdd-logout-revocation-tenant";

fn redis_conn() -> redis::Connection {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    redis::Client::open(url.as_str())
        .expect("redis client")
        .get_connection()
        .expect("redis connection")
}

fn logout_request(jti: &str, exp: u64) -> TypedHandlerRequest<LogoutRequest> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/logout".to_string(),
        handler_name: "auth_logout".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: LogoutRequest {
            refresh_token: None,
            x_tenant_id: TEST_TENANT.to_string(),
        },
        jwt_claims: Some(serde_json::json!({
            "sub": "00000000-0000-0000-0000-000000000001",
            "tenant_id": TEST_TENANT,
            "jti": jti,
            "exp": exp,
        })),
    }
}

/// Scenario: logging out denylists the presented access token's jti.
#[test]
fn logout_denylists_access_jti() {
    if !infra_available() {
        println!("SKIP: Postgres and/or Redis not available");
        return;
    }

    let jti = format!("bdd-access-{}", uuid::Uuid::new_v4());
    let exp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 300;

    let mut conn = redis_conn();
    let before: Option<String> = redis::cmd("GET")
        .arg(format!("denylist:{jti}"))
        .query(&mut conn)
        .unwrap();
    assert!(before.is_none(), "jti must not be denylisted before logout");

    let resp = auth_logout::handle(logout_request(&jti, exp));
    assert!(resp.error.is_empty(), "logout should succeed: {resp:?}");

    let after: Option<String> = redis::cmd("GET")
        .arg(format!("denylist:{jti}"))
        .query(&mut conn)
        .unwrap();
    assert_eq!(
        after.as_deref(),
        Some("revoked"),
        "access jti must be denylisted after logout"
    );

    // TTL must be positive and bounded by the token's remaining lifetime.
    let ttl: i64 = redis::cmd("TTL")
        .arg(format!("denylist:{jti}"))
        .query(&mut conn)
        .unwrap();
    assert!(ttl > 0 && ttl <= 300, "denylist TTL out of range: {ttl}");
}
