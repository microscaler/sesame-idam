//! JWT enrichment via authz-core: verifies the `brrtrouter::http` client path by
//! serving a mock `/authz/principals/effective` on a local port and checking
//! the roles land in the login response and the signed token's sx claims.
//!
//! Requires live Postgres (register/login); skips gracefully without it.

use std::io;
use std::net::TcpStream;
use std::sync::Once;
use std::time::Duration;

use brrtrouter::typed::TypedHandlerRequest;
use http::Method;
use may_minihttp::{HttpServer, HttpService, Request as MiniRequest, Response as MiniResponse};

use sesame_idam_identity_login_service::controllers::{auth_login, auth_register};
use sesame_idam_identity_login_service::services::authz_client::AUTHZ_CORE_URL_ENV;
use sesame_idam_identity_login_service_gen::handlers::auth_login::Request as LoginRequest;
use sesame_idam_identity_login_service_gen::handlers::auth_register::Request as RegisterRequest;

const TEST_TENANT: &str = "bdd-enrich-tenant";
const MOCK_PORT: u16 = 18102;

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
        // Each nextest test runs in its own process with its own pool —
        // keep pools tiny so parallel DB tests don't exhaust Postgres
        // max_connections.
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

/// Mock authz-core: every POST returns an `EffectiveResponse` with OWNER.
#[derive(Clone)]
struct MockAuthzCore;

impl HttpService for MockAuthzCore {
    fn call(&mut self, _req: MiniRequest, rsp: &mut MiniResponse) -> io::Result<()> {
        rsp.header("Content-Type: application/json");
        rsp.body(
            r#"{"user_id":"ignored","permissions":[],"roles":[{"role":"OWNER","inherited":false}]}"#,
        );
        Ok(())
    }
}

fn register_request(email: &str) -> TypedHandlerRequest<RegisterRequest> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/register".to_string(),
        handler_name: "auth_register".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: RegisterRequest {
            email: email.to_string(),
            first_name: None,
            last_name: None,
            password: "SecureP@ss123!".to_string(),
            phone: None,
            username: None,
            x_tenant_id: TEST_TENANT.to_string(),
        },
    }
}

fn login_request(email: &str) -> TypedHandlerRequest<LoginRequest> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/login".to_string(),
        handler_name: "auth_login".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: LoginRequest {
            email: email.to_string(),
            organization_id: None,
            password: "SecureP@ss123!".to_string(),
            x_tenant_id: TEST_TENANT.to_string(),
        },
    }
}

/// Scenario: Roles fetched from authz-core land in the login response and
/// in the signed token's namespaced sx claims.
#[test]
fn login_enriches_roles_from_authz_core() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    // Given — a mock authz-core listening locally
    let server = HttpServer(MockAuthzCore)
        .start(("127.0.0.1", MOCK_PORT))
        .expect("start mock authz-core");
    std::env::set_var(AUTHZ_CORE_URL_ENV, format!("http://127.0.0.1:{MOCK_PORT}"));

    // And a registered user
    let email = format!("bddtest_enrich_{}@example.com", uuid::Uuid::new_v4());
    let resp = auth_register::handle(register_request(&email));
    assert_eq!(resp.status, 201, "register: {:?}", resp.body);

    // When — logging in
    let resp = auth_login::handle(login_request(&email));
    assert_eq!(resp.status, 200, "login: {:?}", resp.body);

    // Then — roles are in the response body ...
    let roles: Vec<&str> = resp.body["roles"]
        .as_array()
        .expect("roles array")
        .iter()
        .filter_map(|r| r.as_str())
        .collect();
    assert_eq!(
        roles,
        vec!["OWNER"],
        "roles from authz-core must be returned"
    );

    // ... and inside the signed token's namespaced claims
    let token = resp.body["access_token"].as_str().expect("access_token");
    let payload_b64 = token.split('.').nth(1).expect("payload");
    let payload: serde_json::Value = {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload_b64).unwrap()).unwrap()
    };
    assert_eq!(
        payload["https://sesame-idam.dev/claims"]["roles"],
        serde_json::json!(["OWNER"]),
        "roles must be embedded in sx claims"
    );

    // The mock server coroutine dies with the test process; explicit
    // cancellation is unsafe in may and unnecessary here.
    drop(server);
}

/// Scenario: authz-core unreachable — login still succeeds with empty roles
/// (graceful degradation; enrichment is best-effort at login time).
#[test]
fn login_degrades_gracefully_without_authz_core() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    // Point at a port nothing listens on.
    std::env::set_var(AUTHZ_CORE_URL_ENV, "http://127.0.0.1:1");

    let email = format!("bddtest_noauthz_{}@example.com", uuid::Uuid::new_v4());
    let resp = auth_register::handle(register_request(&email));
    assert_eq!(resp.status, 201);

    let resp = auth_login::handle(login_request(&email));
    assert_eq!(
        resp.status, 200,
        "login must not fail when authz-core is down: {:?}",
        resp.body
    );
    assert_eq!(resp.body["roles"], serde_json::json!([]));
}
