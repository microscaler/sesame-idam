//! Track A contract tests — OpenAPI path shape + gen/doc sync (detect spec drift).

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_repo_file(rel: &str) -> String {
    let path = repo_root().join(rel);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

const ORG_MGMT_SAML_PATHS: &[&str] = &[
    "/organizations/{org_id}/sso/saml",
    "/organizations/{org_id}/sso/saml/allow",
    "/organizations/{org_id}/sso/saml/disable",
    "/organizations/{org_id}/sso/saml/enable",
    "/organizations/{org_id}/sso/saml/link",
    "/organizations/{org_id}/sso/saml/metadata",
];

const LOGIN_SAML_PATHS: &[&str] = &[
    "/auth/saml/login",
    "/auth/saml/callback",
];

/// Stale `/sso/saml/*` without org segment must not reappear (routing/codegen drift).
const FORBIDDEN_ORG_MGMT_PATHS: &[&str] = &[
    "  /sso/saml:",
    "  /sso/saml/allow:",
    "  /sso/saml/disable:",
    "  /sso/saml/enable:",
    "  /sso/saml/link:",
    "  /sso/saml/metadata:",
];

#[test]
fn org_mgmt_openapi_saml_paths_are_org_scoped() {
    let spec = read_repo_file("openapi/idam/org-mgmt/openapi.yaml");
    for path in ORG_MGMT_SAML_PATHS {
        assert!(
            spec.contains(path),
            "org-mgmt OpenAPI missing SAML path {path}"
        );
    }
    for stale in FORBIDDEN_ORG_MGMT_PATHS {
        assert!(
            !spec.contains(stale),
            "org-mgmt OpenAPI still has stale path marker {stale:?}"
        );
    }
}

#[test]
fn identity_login_openapi_exposes_saml_facade() {
    let spec = read_repo_file("openapi/idam/identity-login-service/openapi.yaml");
    for path in LOGIN_SAML_PATHS {
        assert!(
            spec.contains(path),
            "identity-login OpenAPI missing SAML path {path}"
        );
    }
    assert!(
        spec.contains("operationId: saml_login"),
        "missing saml_login operationId"
    );
    assert!(
        spec.contains("operationId: saml_callback"),
        "missing saml_callback operationId"
    );
}

#[test]
fn org_mgmt_gen_openapi_matches_canonical_spec() {
    let src = read_repo_file("openapi/idam/org-mgmt/openapi.yaml");
    let gen = read_repo_file("microservices/idam/org-mgmt/gen/doc/openapi.yaml");
    assert_eq!(
        src, gen,
        "org-mgmt gen/doc/openapi.yaml drift — run `just gen-org-mgmt`"
    );
}

#[test]
fn identity_login_gen_openapi_matches_canonical_spec() {
    let src = read_repo_file("openapi/idam/identity-login-service/openapi.yaml");
    let gen = read_repo_file("microservices/idam/identity-login-service/gen/doc/openapi.yaml");
    assert_eq!(
        src, gen,
        "identity-login gen/doc/openapi.yaml drift — run `just gen-identity-login`"
    );
}
