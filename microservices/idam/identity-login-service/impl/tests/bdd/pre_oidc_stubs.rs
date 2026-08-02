//! Contract coverage for pre-OIDC paths that are still stubbed / unwired.
//!
//! Dual OTP send/verify and SMS magic-link verify are present in OpenAPI but
//! not registered in `controllers/mod.rs`. These tests lock that inventory so
//! we do not silently claim e2e coverage for unfinished surfaces.

use brrtrouter::load_spec;
use std::path::PathBuf;

fn login_routes() -> Vec<brrtrouter::spec::RouteMeta> {
    let spec = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../openapi/idam/identity-login-service/openapi.yaml")
        .canonicalize()
        .expect("openapi path");
    let (routes, _) = load_spec(spec.to_str().unwrap()).expect("load openapi");
    routes
}

fn has_handler(routes: &[brrtrouter::spec::RouteMeta], handler: &str) -> bool {
    routes.iter().any(|r| r.handler_name.as_ref() == handler)
}

#[test]
fn openapi_still_declares_dual_otp_and_sms_magic_verify() {
    let routes = login_routes();
    for handler in [
        "login_dual_otp",
        "verify_dual_otp",
        "sms_magic_link_send",
        "sms_magic_link_verify",
    ] {
        assert!(
            has_handler(&routes, handler),
            "{handler} must remain in OpenAPI until implemented or explicitly removed"
        );
    }
}

#[test]
fn dual_otp_and_sms_magic_verify_are_public_north_routes() {
    let routes = login_routes();
    for handler in ["login_dual_otp", "verify_dual_otp", "sms_magic_link_verify"] {
        let security = routes
            .iter()
            .find(|r| r.handler_name.as_ref() == handler)
            .expect(handler)
            .security
            .clone();
        assert!(
            security.is_empty(),
            "{handler} must stay public (security: []) — got {security:?}"
        );
    }
}

/// Source-level inventory: unfinished controller files exist but are not
/// declared in `controllers/mod.rs` (Register & Overwrite ADR).
#[test]
fn dual_otp_controllers_are_not_wired_in_mod() {
    let mod_rs = include_str!("../../src/controllers/mod.rs");
    assert!(
        !mod_rs.contains("pub mod login_dual_otp"),
        "login_dual_otp must stay unwired until implemented against Redis OTP"
    );
    assert!(
        !mod_rs.contains("pub mod verify_dual_otp"),
        "verify_dual_otp must stay unwired until implemented"
    );
    assert!(
        !mod_rs.contains("pub mod sms_magic_link_verify"),
        "sms_magic_link_verify must stay unwired until provider + token mint exist"
    );
    assert!(
        mod_rs.contains("pub mod sms_magic_link_send"),
        "sms_magic_link_send is partially implemented (abuse gate) and must stay wired"
    );
}
