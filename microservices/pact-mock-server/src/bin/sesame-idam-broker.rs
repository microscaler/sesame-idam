//! Sesame-IDAM local pact broker — SAML SSO + OAuth provider mocks.
//!
//! Listens on port 9190 by default. No public internet required.
//!
//! ```bash
//! cargo run -p pact-mock-server --bin sesame-idam-broker
//! ```

use pact_mock_server::sesame_idam_broker::{build_app, BrokerState, DEFAULT_PORT};
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let port: u16 = std::env::var("SESAME_BROKER_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let base_url = std::env::var("SESAME_BROKER_BASE_URL")
        .unwrap_or_else(|_| format!("http://127.0.0.1:{port}"));
    let app_redirect = std::env::var("SESAME_BROKER_APP_REDIRECT_URL").unwrap_or_else(|_| {
        "http://hauliage.dev.microscaler.local/saml/callback".to_string()
    });

    let state = BrokerState::new(base_url, app_redirect);
    let app = build_app(state);

    let addr = format!("0.0.0.0:{port}");
    info!("Sesame IdAM pact broker listening on {addr}");
    info!("  SAML redirect: POST /v1/saml/redirect");
    info!("  SAML redeem:   POST /v1/saml/redeem");
    info!("  IdP simulate:  POST /idp/simulate (CI)");
    info!("  Google OAuth:  /mock/google/*");
    info!("  Microsoft:     /mock/microsoft/*");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("bind sesame-idam-broker");
    axum::serve(listener, app)
        .await
        .expect("serve sesame-idam-broker");
}
