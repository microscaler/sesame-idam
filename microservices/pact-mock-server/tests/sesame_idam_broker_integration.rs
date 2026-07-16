//! Integration tests for the Sesame IdAM pact broker (SAML + OAuth mocks).

use axum_test::TestServer;
use pact_mock_server::sesame_idam_broker::BrokerState;
use serde_json::json;

fn test_server() -> TestServer {
    let state = BrokerState::new("http://127.0.0.1:9190", "http://localhost:7174/saml/callback");
    let app = pact_mock_server::sesame_idam_broker::build_app(state);
    TestServer::new(app).expect("test server")
}

const API_KEY: &str = "ssoready_sk_test";

#[tokio::test]
async fn health_returns_ok() {
    let server = test_server();
    let response = server.get("/health").await;
    assert_eq!(response.status_code(), 200);
}

#[tokio::test]
async fn saml_redirect_requires_api_key() {
    let server = test_server();
    let response = server
        .post("/v1/saml/redirect")
        .json(&json!({ "organizationExternalId": "a1000001-0001-4000-8000-000000000001" }))
        .await;
    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
async fn saml_redirect_and_redeem_round_trip() {
    let server = test_server();

    let redirect = server
        .post("/v1/saml/redirect")
        .add_header("Authorization", format!("Bearer {API_KEY}"))
        .json(&json!({ "organizationExternalId": "a1000001-0001-4000-8000-000000000001" }))
        .await;
    assert_eq!(redirect.status_code(), 200);
    let redirect_body: serde_json::Value = redirect.json();
    assert!(redirect_body["redirectUrl"]
        .as_str()
        .unwrap_or("")
        .contains("/idp/login"));

    let simulate = server
        .post("/idp/simulate")
        .json(&json!({
            "organizationExternalId": "a1000001-0001-4000-8000-000000000001",
            "email": "buyer@testshipper.local"
        }))
        .await;
    assert_eq!(simulate.status_code(), 200);
    let sim_body: serde_json::Value = simulate.json();
    let code = sim_body["samlAccessCode"].as_str().expect("access code");
    assert!(code.starts_with("saml_access_code_"));

    let redeem = server
        .post("/v1/saml/redeem")
        .add_header("Authorization", format!("Bearer {API_KEY}"))
        .json(&json!({ "samlAccessCode": code }))
        .await;
    assert_eq!(redeem.status_code(), 200);
    let redeem_body: serde_json::Value = redeem.json();
    assert_eq!(redeem_body["email"], "buyer@testshipper.local");
    assert_eq!(
        redeem_body["organizationExternalId"],
        "a1000001-0001-4000-8000-000000000001"
    );

    // Single-use: second redeem fails
    let again = server
        .post("/v1/saml/redeem")
        .add_header("Authorization", format!("Bearer {API_KEY}"))
        .json(&json!({ "samlAccessCode": code }))
        .await;
    assert_eq!(again.status_code(), 400);
}

/// Hauliage AME Corp shipper org — broker simulate + redeem (Track A MVP org id).
#[tokio::test]
async fn ame_corp_broker_simulate_and_redeem() {
    let server = test_server();
    const AME_CORP: &str = "b2000002-0002-4000-8000-000000000002";
    const SHIPPER: &str = "shipper@amecorp.dev";

    let simulate = server
        .post("/idp/simulate")
        .json(&json!({
            "organizationExternalId": AME_CORP,
            "email": SHIPPER,
        }))
        .await;
    assert_eq!(simulate.status_code(), 200);
    let sim_body: serde_json::Value = simulate.json();
    let code = sim_body["samlAccessCode"].as_str().expect("access code");

    let redeem = server
        .post("/v1/saml/redeem")
        .add_header("Authorization", format!("Bearer {API_KEY}"))
        .json(&json!({ "samlAccessCode": code }))
        .await;
    assert_eq!(redeem.status_code(), 200);
    let redeem_body: serde_json::Value = redeem.json();
    assert_eq!(redeem_body["email"], SHIPPER);
    assert_eq!(redeem_body["organizationExternalId"], AME_CORP);
}

#[tokio::test]
async fn pact_fixture_access_code_redeems() {
    let server = test_server();
    let response = server
        .post("/v1/saml/redeem")
        .add_header("Authorization", format!("Bearer {API_KEY}"))
        .json(&json!({ "samlAccessCode": "saml_access_code_test" }))
        .await;
    assert_eq!(response.status_code(), 200);
    let body: serde_json::Value = response.json();
    assert_eq!(body["email"], "buyer@testshipper.local");
}

#[tokio::test]
async fn google_oauth_token_and_userinfo() {
    let server = test_server();

    let token = server
        .post("/mock/google/token")
        .content_type("application/x-www-form-urlencoded")
        .text("grant_type=authorization_code&code=google_test_code&client_id=test&client_secret=test&redirect_uri=http%3A%2F%2Flocal%2Fcb")
        .await;
    assert_eq!(token.status_code(), 200);
    let token_body: serde_json::Value = token.json();
    let access = token_body["access_token"].as_str().unwrap();

    let profile = server
        .get("/mock/google/userinfo")
        .add_header("Authorization", format!("Bearer {access}"))
        .await;
    assert_eq!(profile.status_code(), 200);
    let profile_body: serde_json::Value = profile.json();
    assert_eq!(profile_body["email"], "alice@gmail.com");
    assert_eq!(profile_body["email_verified"], true);
}

#[tokio::test]
async fn microsoft_oauth_token_and_me() {
    let server = test_server();

    let token = server
        .post("/mock/microsoft/token")
        .content_type("application/x-www-form-urlencoded")
        .text("grant_type=authorization_code&code=microsoft_test_code&client_id=test&client_secret=test&redirect_uri=http%3A%2F%2Flocal%2Fcb")
        .await;
    assert_eq!(token.status_code(), 200);

    let me = server
        .get("/mock/microsoft/me")
        .add_header("Authorization", "Bearer microsoft_mock_access_token")
        .await;
    assert_eq!(me.status_code(), 200);
    let body: serde_json::Value = me.json();
    assert_eq!(body["mail"], "bob@outlook.com");
}

#[tokio::test]
async fn google_authorize_redirects_with_code() {
    let server = test_server();
    let response = server
        .get("/mock/google/o/oauth2/v2/auth?redirect_uri=http%3A%2F%2Flocal%2Foauth%2Fcallback&state=csrf-1&email=alice%40gmail.com")
        .await;
    assert_eq!(response.status_code(), 307);
    let location = response
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(location.contains("code=google_code_"));
    assert!(location.contains("state=csrf-1"));
}
