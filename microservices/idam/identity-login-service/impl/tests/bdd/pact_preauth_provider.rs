//! Pact provider verification for Series A P0/P1 pre-auth north–south.
//!
//! Loads `Sesame-Identity-Login-PreAuth.json` and replays each interaction over
//! real HTTP against a provider base URL:
//!
//! - `SESAME_PACT_PROVIDER_BASE` (preferred) — e.g. `http://127.0.0.1:18081/idam/v1`
//!   when running the freshly built login binary locally
//! - else `https://api.sesameidentity.dev.local/idam/v1` when reachable
//!
//! Skips when no provider base is reachable.

use std::fs;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;

use crate::common::ensure_fixture_web_client;

fn pact_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../pact-mock-server/pacts/Sesame-Identity-Login-PreAuth.json")
}

fn provider_base() -> Option<String> {
    if let Ok(base) = std::env::var("SESAME_PACT_PROVIDER_BASE") {
        let base = base.trim_end_matches('/').to_string();
        if base_reachable(&base) {
            return Some(base);
        }
        eprintln!("SESAME_PACT_PROVIDER_BASE set but unreachable: {base}");
        return None;
    }
    let north = "https://api.sesameidentity.dev.local/idam/v1".to_string();
    if base_reachable(&north) {
        return Some(north);
    }
    None
}

fn base_reachable(base: &str) -> bool {
    let stripped = base
        .strip_prefix("https://")
        .or_else(|| base.strip_prefix("http://"))
        .unwrap_or(base);
    let hostport = stripped.split('/').next().unwrap_or(stripped);
    let (host, port) = match hostport.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().unwrap_or(443)),
        None => {
            if base.starts_with("https://") {
                (hostport, 443u16)
            } else {
                (hostport, 80u16)
            }
        }
    };
    if host == "127.0.0.1" || host == "localhost" {
        return TcpStream::connect_timeout(
            &SocketAddr::from(([127, 0, 0, 1], port)),
            Duration::from_millis(400),
        )
        .is_ok();
    }
    let Ok(mut addrs) = (host, port).to_socket_addrs() else {
        return false;
    };
    addrs.any(|addr| TcpStream::connect_timeout(&addr, Duration::from_millis(800)).is_ok())
}

fn http_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(15))
        .build()
        .expect("http client")
}

fn load_pact() -> Value {
    let text = fs::read_to_string(pact_path()).unwrap_or_else(|e| {
        panic!("missing Pact contract at {}: {e}", pact_path().display())
    });
    serde_json::from_str(&text).expect("Sesame-Identity-Login-PreAuth.json must be valid JSON")
}

fn assert_body_subset(actual: &Value, expected: &Value, description: &str) {
    match expected {
        Value::Object(exp_map) => {
            let actual_map = actual
                .as_object()
                .unwrap_or_else(|| panic!("{description}: expected JSON object, got {actual}"));
            for (key, exp_val) in exp_map {
                let act_val = actual_map.get(key).unwrap_or_else(|| {
                    panic!("{description}: missing response field `{key}` in {actual}")
                });
                assert_eq!(act_val, exp_val, "{description}: field `{key}` mismatch");
            }
        }
        other => panic!("{description}: expected object body matcher, got {other}"),
    }
}

fn verify_interaction(client: &reqwest::blocking::Client, base: &str, interaction: &Value) {
    let description = interaction["description"]
        .as_str()
        .unwrap_or("unnamed interaction");
    let request = &interaction["request"];
    let expected = &interaction["response"];

    let method = request["method"].as_str().expect("request.method");
    let path = request["path"].as_str().expect("request.path");
    let query = request.get("query").and_then(Value::as_str).unwrap_or("");
    let url = if query.is_empty() {
        format!("{base}{path}")
    } else {
        format!("{base}{path}?{query}")
    };

    let mut builder = match method {
        "GET" => client.get(&url),
        "POST" => client.post(&url),
        other => panic!("{description}: unsupported method {other}"),
    };

    if let Some(headers) = request.get("headers").and_then(Value::as_object) {
        for (name, value) in headers {
            if let Some(v) = value.as_str() {
                builder = builder.header(name, v);
            }
        }
        assert!(
            !headers.keys().any(|k| k.eq_ignore_ascii_case("X-Tenant-ID")),
            "{description}: Pact request must omit X-Tenant-ID"
        );
    }

    if let Some(body) = request.get("body") {
        builder = builder.json(body);
    }

    let response = builder
        .send()
        .unwrap_or_else(|e| panic!("{description}: HTTP request failed: {e}"));

    let expected_status = expected["status"].as_u64().expect("response.status") as u16;
    let status = response.status().as_u16();
    let text = response.text().unwrap_or_default();
    assert_eq!(
        status, expected_status,
        "{description}: status mismatch; body={text}"
    );

    if let Some(expected_body) = expected.get("body") {
        let actual_body: Value = serde_json::from_str(&text).unwrap_or_else(|e| {
            panic!("{description}: response is not JSON ({e}): {text}");
        });
        assert_body_subset(&actual_body, expected_body, description);
    }
}

#[test]
fn pact_file_documents_series_a_p0_p1_preauth() {
    let pact = load_pact();
    assert_eq!(pact["consumer"]["name"], "sesame-idam-client");
    assert_eq!(pact["provider"]["name"], "identity-login-service");
    let interactions = pact["interactions"]
        .as_array()
        .expect("interactions array");
    assert!(
        interactions.len() >= 5,
        "expected forgot/reset/social interactions"
    );

    let descriptions: Vec<&str> = interactions
        .iter()
        .filter_map(|i| i["description"].as_str())
        .collect();
    assert!(descriptions.iter().any(|d| d.contains("forgot password")));
    assert!(descriptions.iter().any(|d| d.contains("reset password")));
    assert!(descriptions.iter().any(|d| d.contains("social login")));
}

#[test]
fn provider_verifies_preauth_pact_over_http() {
    let Some(base) = provider_base() else {
        eprintln!(
            "SKIP pact provider verify: set SESAME_PACT_PROVIDER_BASE \
             (e.g. http://127.0.0.1:18081/idam/v1) or expose the north API"
        );
        return;
    };
    eprintln!("pact provider verify against {base}");

    // Provider states: registered public client acme-web for tenant acme.
    // Configure the same DB the provider process uses (DB_HOST/DB_PORT/…).
    std::env::set_var("DB_POOL_MAX", "2");
    ensure_fixture_web_client();

    let pact = load_pact();
    let client = http_client();
    let interactions = pact["interactions"]
        .as_array()
        .expect("interactions array");

    for interaction in interactions {
        verify_interaction(&client, &base, interaction);
    }
}
