// User-owned controller for handler 'fetch_org'.

use crate::handlers::fetch_org::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(FetchOrgController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        can_setup_saml: Some(true),
        created_at: "example".to_string(),
        domain: Some("example".to_string()),
        domain_auto_join: Some(true),
        domain_restrict: Some(true),
        domains: Some(vec![]),
        id: "example".to_string(),
        is_saml_configured: Some(true),
        is_saml_in_test_mode: Some(true),
        isolated: Some(true),
        legacy_org_id: Some(Default::default()),
        logo_url: Some("example".to_string()),
        max_users: Some(Default::default()),
        metadata: Some(Default::default()),
        name: "example".to_string(),
        password_rotation_enabled: Some(true),
        password_rotation_history_size: Some(42),
        password_rotation_period: Some(42),
        slug: "example".to_string(),
        sso_trust_level: Some(Default::default()),
        updated_at: Some("example".to_string()),
    }
}
