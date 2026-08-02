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

    for handler in [
        "auth_login",
        "auth_register",
        "login_email_otp",
        "verify_email_otp",
        "login_phone_otp",
        "verify_phone_otp",
        "magic_link_send",
        "magic_link_verify",
        "sms_magic_link_send",
        "auth_token",
        "auth_forgot_password",
        "auth_reset_password",
        "signup_validate",
    ] {
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
fn login_spec_platform_routes_require_platform_service_auth() {
    let routes = load_idam_routes("openapi/idam/identity-login-service/openapi.yaml");

    for handler in [
        "platform_tenant_create",
        "platform_tenant_get",
        "platform_tenant_status_patch",
        "platform_tenant_oauth_upsert",
        "platform_tenant_oauth_rotate",
    ] {
        let security = security_for(&routes, handler);
        assert!(
            security
                .iter()
                .any(|req| req.0.contains_key("PlatformServiceAuth")),
            "{handler} must require PlatformServiceAuth (not global BearerAuth)"
        );
    }
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

#[test]
fn login_spec_oidc_authorize_and_token_are_public() {
    let routes = load_idam_routes("openapi/idam/identity-login-service/openapi.yaml");

    for handler in ["oauth_authorize", "oauth_token"] {
        assert!(
            security_for(&routes, handler).is_empty(),
            "{handler} must be public (security: []) — client_id/secret authenticate the request"
        );
    }
}

#[test]
fn login_spec_oauth_userinfo_requires_bearer() {
    let routes = load_idam_routes("openapi/idam/identity-login-service/openapi.yaml");
    let security = security_for(&routes, "oauth_userinfo");
    assert!(
        security.iter().any(|req| req.0.contains_key("BearerAuth")),
        "oauth_userinfo must require BearerAuth"
    );
}

#[test]
fn login_spec_error_response_includes_oauth_error_codes() {
    let spec_path = repo_root().join("openapi/idam/identity-login-service/openapi.yaml");
    let raw = std::fs::read_to_string(&spec_path).expect("read login openapi");
    let doc: serde_yaml::Value = serde_yaml::from_str(&raw).expect("parse login openapi");
    let enum_values = doc
        .get("components")
        .and_then(|c| c.get("schemas"))
        .and_then(|s| s.get("ErrorResponse"))
        .and_then(|e| e.get("properties"))
        .and_then(|p| p.get("error"))
        .and_then(|e| e.get("enum"))
        .and_then(|v| v.as_sequence())
        .expect("ErrorResponse.error.enum");
    let codes: Vec<&str> = enum_values.iter().filter_map(|v| v.as_str()).collect();

    for required in [
        "invalid_client",
        "unauthorized_client",
        "unsupported_grant_type",
        "unsupported_response_type",
        "invalid_scope",
        "invalid_grant",
        "temporarily_unavailable",
        "access_denied",
    ] {
        assert!(
            codes.contains(&required),
            "ErrorResponse enum missing OAuth code {required}; got {codes:?}"
        );
    }
}
