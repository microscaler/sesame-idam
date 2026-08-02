//! BDD tests for OIDC discovery (H6.5) — populated discovery document.

use brrtrouter::typed::TypedHandlerRequest;
use http::Method;
use sesame_idam_identity_session_service::controllers::openid_configuration;
use sesame_idam_identity_session_service_gen::handlers::openid_configuration::Request;

#[test]
fn openid_configuration_returns_populated_document() {
    let req = TypedHandlerRequest {
        method: Method::GET,
        path: "/.well-known/openid-configuration".to_string(),
        handler_name: "openid_configuration".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: Request {},
        jwt_claims: None,
    };

    let resp = openid_configuration::handle(req);

    assert!(resp.issuer.as_ref().is_some_and(|s| !s.is_empty()));
    assert!(resp.jwks_uri.as_ref().is_some_and(|s| s.contains("jwks")));
    assert!(resp
        .token_endpoint
        .as_ref()
        .is_some_and(|s| s == "https://api.sesameidentity.dev.local/oauth/token"));
    assert!(resp
        .userinfo_endpoint
        .as_ref()
        .is_some_and(|s| s.contains("userinfo")));
    assert!(resp
        .scopes_supported
        .as_ref()
        .is_some_and(|s| s.contains(&"openid".to_string())));
    assert!(resp
        .id_token_signing_alg_values_supported
        .as_ref()
        .is_some_and(|a| a.contains(&"EdDSA".to_string())));
    assert_eq!(
        resp.response_types_supported.as_deref(),
        Some(&["code".to_string()][..])
    );
    assert_eq!(
        resp.grant_types_supported.as_deref(),
        Some(
            &[
                "authorization_code".to_string(),
                "refresh_token".to_string()
            ][..]
        )
    );
    assert_eq!(
        resp.code_challenge_methods_supported.as_deref(),
        Some(&["S256".to_string()][..])
    );
    assert_eq!(
        resp.token_endpoint_auth_methods_supported.as_deref(),
        Some(
            &[
                "none".to_string(),
                "client_secret_basic".to_string(),
                "client_secret_post".to_string()
            ][..]
        )
    );
}
