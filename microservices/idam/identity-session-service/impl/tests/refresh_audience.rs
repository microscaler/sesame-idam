//! A refreshed access token must carry the same audiences as the login token.
//!
//! Rotation used to mint `aud = ["sesame-idam"]` only, so every service checking its own audience (the session
//! service's `/identity/me` among them) refused the refreshed token and the portal signed the user out about one
//! access-token lifetime after sign-in.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

use sesame_idam_identity_session_service::models::refresh_token::RefreshToken;
use sesame_idam_identity_session_service::services::token_issuer::issue_rotated_tokens;

#[test]
fn rotated_access_token_keeps_every_service_audience() {
    // Development key, and unreachable Redis / authz-core: issuance degrades (ver=1, no roles) but still signs.
    std::env::set_var("SESAME_DEV_MODE", "1");
    std::env::set_var("REDIS_URL", "redis://127.0.0.1:1");
    std::env::set_var("AUTHZ_CORE_URL", "http://127.0.0.1:1");
    std::env::remove_var("JWT_ISSUE_AUDIENCES");

    let now = chrono::Utc::now().timestamp();
    let stored = RefreshToken::new_with_tenant(
        "jti-1".into(),
        "user-1".into(),
        "sid-1".into(),
        "family-1".into(),
        now,
        now + 3600,
        "platform".into(),
        "pricewhisperer".into(),
        "openid".into(),
    );
    let issued = issue_rotated_tokens(&stored, "pricewhisperer", 1).expect("rotated tokens");

    let payload = issued.access_token.split('.').nth(1).expect("JWT payload");
    let claims: serde_json::Value =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).expect("base64url"))
            .expect("JSON claims");
    let aud: Vec<String> = serde_json::from_value(claims["aud"].clone()).expect("aud array");

    assert_eq!(aud, sesame_common::deployment::issue_audiences());
    for service in [
        "identity-session",
        "authz-core",
        "org-mgmt",
        "identity-user-mgmt",
        "api-keys",
    ] {
        assert!(
            aud.iter().any(|a| a == service),
            "refreshed token lacks aud {service}: {aud:?}"
        );
    }
}
