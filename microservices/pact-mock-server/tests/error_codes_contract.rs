//! Every error code a service's handlers answer is listed in its `ErrorResponse` enum.
//!
//! BRRTRouter validates every JSON response against the spec, so a handler answering a code the enum does not list
//! fails with a 500 "Response validation failed" instead of its own status (seen in pp: a 409 `slug_taken` became a
//! 500). This scans each service's handlers for `"error": "<code>"` literals.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Services whose `ErrorResponse.error` is an enum.
const SERVICES: &[&str] = &[
    "identity-login-service",
    "identity-session-service",
    "identity-user-mgmt-service",
    "api-keys",
    "authz-core",
];

fn listed_codes(service: &str) -> Vec<String> {
    let path = repo_root().join(format!("openapi/idam/{service}/openapi.yaml"));
    let doc: serde_yaml::Value =
        serde_yaml::from_str(&std::fs::read_to_string(&path).expect("read spec"))
            .expect("parse spec");
    doc["components"]["schemas"]["ErrorResponse"]["properties"]["error"]["enum"]
        .as_sequence()
        .unwrap_or_else(|| panic!("{service}: ErrorResponse.error has no enum"))
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect()
}

fn answered_codes(dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    const NEEDLE: &str = "\"error\": \"";
    for entry in std::fs::read_dir(dir).expect("read dir").flatten() {
        let path = entry.path();
        if path.is_dir() {
            answered_codes(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            let text = std::fs::read_to_string(&path).expect("read source");
            for (i, _) in text.match_indices(NEEDLE) {
                let rest = &text[i + NEEDLE.len()..];
                let code: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_lowercase() || *c == '_')
                    .collect();
                if !code.is_empty() && rest[code.len()..].starts_with('"') {
                    out.push((code, path.clone()));
                }
            }
        }
    }
}

#[test]
fn every_error_code_a_handler_answers_is_in_its_spec() {
    let mut missing = Vec::new();
    for service in SERVICES {
        let listed = listed_codes(service);
        let mut answered = Vec::new();
        answered_codes(
            &repo_root().join(format!("microservices/idam/{service}/impl/src")),
            &mut answered,
        );
        for (code, path) in answered {
            if !listed.contains(&code) {
                missing.push(format!("{service}: {code} ({})", path.display()));
            }
        }
    }
    missing.sort();
    missing.dedup();
    assert!(
        missing.is_empty(),
        "error codes missing from ErrorResponse: {missing:#?}"
    );
}
