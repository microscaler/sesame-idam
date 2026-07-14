//! `OpenAPI` security inheritance regression (BR-1 / SI-2).
//!
//! Ensures login/session specs keep public auth routes (`security: []`) while
//! protected routes inherit or declare `BearerAuth` when global security is set.

use brrtrouter::load_spec;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../..")
        .canonicalize()
        .expect("repo root")
}

fn load_idam_routes(relative_spec: &str) -> Vec<brrtrouter::spec::RouteMeta> {
    let spec_path = repo_root().join(relative_spec);
    let (routes, _) = load_spec(spec_path.to_str().unwrap()).unwrap_or_else(|e| {
        panic!("failed to load {}: {e}", spec_path.display());
    });
    routes
}

fn security_for(
    routes: &[brrtrouter::spec::RouteMeta],
    handler: &str,
) -> Vec<brrtrouter::spec::SecurityRequirement> {
    routes
        .iter()
        .find(|r| r.handler_name.as_ref() == handler)
        .unwrap_or_else(|| panic!("handler {handler} not in spec"))
        .security
        .clone()
}

#[test]
fn login_spec_public_routes_have_no_security_with_global_default() {
    let routes = load_idam_routes("openapi/idam/identity-login-service/openapi.yaml");

    for handler in ["auth_login", "auth_register"] {
        assert!(
            security_for(&routes, handler).is_empty(),
            "{handler} must remain public (security: []) when global security is set"
        );
    }
}

#[test]
fn login_spec_logout_inherits_global_bearer() {
    let routes = load_idam_routes("openapi/idam/identity-login-service/openapi.yaml");

    let logout = security_for(&routes, "auth_logout");
    assert!(
        logout.iter().any(|req| req.0.contains_key("BearerAuth")),
        "auth_logout must inherit global BearerAuth"
    );
}
#[test]
fn login_spec_explicit_bearer_routes_require_bearer() {
    let routes = load_idam_routes("openapi/idam/identity-login-service/openapi.yaml");

    let profile = security_for(&routes, "get_user_profile");
    assert!(
        profile.iter().any(|req| req.0.contains_key("BearerAuth")),
        "get_user_profile must require BearerAuth"
    );
}

#[test]
fn session_spec_public_discovery_routes_have_no_security() {
    let routes = load_idam_routes("openapi/idam/identity-session-service/openapi.yaml");

    for handler in ["jwks", "openid_configuration", "auth_refresh"] {
        assert!(
            security_for(&routes, handler).is_empty(),
            "{handler} must be public with global security + security: []"
        );
    }
}
