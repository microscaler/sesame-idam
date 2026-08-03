//! Epic 15.3/15.4 — live contract checks for the public tenant-consumer OpenAPI.
//!
//! Validates that every public operation is documented, examples avoid internal
//! hostnames, and (when live) discovery/register surfaces are reachable on
//! public hosts.

use std::fs;
use std::net::{SocketAddr, ToSocketAddrs, TcpStream};
use std::path::PathBuf;
use std::time::Duration;

use serde_yaml::Value;

const API_BASE: &str = "https://api.sesameidentity.dev.local";
const ID_BASE: &str = "https://id.sesameidentity.dev.local";

const PUBLIC_OPERATIONS: &[(&str, &str, &str)] = &[
    ("post", "/auth/register", "register_user"),
    ("get", "/users/me/memberships", "list_my_memberships"),
    ("post", "/organizations", "create_organization"),
    (
        "post",
        "/organizations/{org_id}/invitations",
        "invite_user_to_organization",
    ),
    ("post", "/invitations/accept", "accept_invitation"),
    ("get", "/invitations/preview", "preview_invitation"),
    (
        "post",
        "/sessions/active-organization",
        "set_active_organization",
    ),
];

fn repo_root() -> PathBuf {
    // impl/ → identity-login-service → idam → microservices → sesame-idam
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../..")
        .canonicalize()
        .expect("repo root")
}

fn load_tenant_consumer() -> Value {
    let path = repo_root().join("openapi/idam/tenant-consumer/openapi.yaml");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_yaml::from_str(&text).expect("parse tenant-consumer openapi")
}

fn live_available() -> bool {
    let host = "api.sesameidentity.dev.local";
    let Ok(mut addrs) = (host, 443u16).to_socket_addrs() else {
        return false;
    };
    let Some(addr) = addrs.next() else {
        return false;
    };
    TcpStream::connect_timeout(&addr, Duration::from_millis(800)).is_ok()
        || TcpStream::connect_timeout(
            &SocketAddr::from(([10, 177, 76, 220], 443)),
            Duration::from_millis(400),
        )
        .is_ok()
}

fn http_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .expect("http client")
}

#[test]
fn tenant_consumer_documents_all_public_operations() {
    let spec = load_tenant_consumer();
    let paths = spec
        .get("paths")
        .and_then(Value::as_mapping)
        .expect("paths");
    for (method, path, operation_id) in PUBLIC_OPERATIONS {
        let actual = paths
            .get(&Value::String((*path).into()))
            .and_then(|item| item.get(*method))
            .and_then(|op| op.get("operationId"))
            .and_then(Value::as_str);
        assert_eq!(
            actual,
            Some(*operation_id),
            "expected {method} {path} → {operation_id}"
        );
    }
}

#[test]
fn tenant_consumer_server_is_public_idam_v1() {
    let spec = load_tenant_consumer();
    let servers = spec
        .get("servers")
        .and_then(Value::as_sequence)
        .expect("servers");
    let url = servers[0]
        .get("url")
        .and_then(Value::as_str)
        .expect("server url");
    assert!(
        url.ends_with("/idam/v1"),
        "public SDK must be under /idam/v1, got {url}"
    );
    assert!(
        !url.contains(".svc.cluster.local"),
        "internal k8s hostname leaked into public OpenAPI"
    );
    assert!(
        !url.contains("identity-login-service"),
        "service hostname leaked into public OpenAPI"
    );
}

#[test]
fn tenant_consumer_forbids_caller_selected_tenancy_header() {
    let text = fs::read_to_string(
        repo_root().join("openapi/idam/tenant-consumer/openapi.yaml"),
    )
    .expect("openapi text");
    assert!(
        !text.contains("X-Tenant-ID:") && !text.contains("name: X-Tenant-ID"),
        "caller-selected X-Tenant-ID must not appear as an API parameter"
    );
    assert!(
        text.contains("must not send `X-Tenant-ID`")
            || text.contains("must not send X-Tenant-ID"),
        "OpenAPI must document that X-Tenant-ID is not a tenancy selector"
    );
}

#[test]
fn tenant_consumer_freezes_transport_schemas() {
    let spec = load_tenant_consumer();
    let schemas = spec
        .get("components")
        .and_then(|c| c.get("schemas"))
        .and_then(Value::as_mapping)
        .expect("components.schemas");
    for schema in ["TokenResponse", "ErrorObject", "MembershipPage"] {
        assert!(
            schemas.contains_key(&Value::String(schema.into())),
            "missing schema {schema}"
        );
    }
    let token = schemas
        .get(&Value::String("TokenResponse".into()))
        .and_then(|s| s.get("required"))
        .and_then(Value::as_sequence)
        .expect("TokenResponse.required");
    for field in ["access_token", "expires_in", "token_type", "user_id"] {
        assert!(
            token.iter().any(|v| v.as_str() == Some(field)),
            "TokenResponse must require {field}"
        );
    }
}

#[test]
fn live_public_api_origin_reaches_idam_prefix() {
    if !live_available() {
        eprintln!("SKIP live tenant-consumer: api host unreachable");
        return;
    }
    let client = http_client();
    // Register without body should not be 404 (route exists on public path).
    let register = client
        .post(format!("{API_BASE}/idam/v1/auth/register"))
        .header("content-type", "application/json")
        .body("{}")
        .send()
        .expect("register probe");
    assert_ne!(
        register.status().as_u16(),
        404,
        "public /idam/v1/auth/register missing from live routing"
    );

    let discovery = client
        .get(format!("{ID_BASE}/.well-known/openid-configuration"))
        .send()
        .expect("discovery");
    assert_eq!(discovery.status(), 200);
    let body = discovery.text().expect("discovery body");
    assert!(
        !body.contains(".svc.cluster.local"),
        "discovery leaked internal hostnames"
    );
}
