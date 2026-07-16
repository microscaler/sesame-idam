use pact_mock_server::searates_graphql::AppState;
use std::env;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let port = env::var("PORT")
        .unwrap_or_else(|_| "8091".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid u16");

    let pact_path = env::var("SEARATES_PACT_FILE").unwrap_or_else(|_| {
        if std::path::Path::new("SEARATES-API.json").exists() {
            "SEARATES-API.json".to_string()
        } else if let Ok(exe_dir) = std::env::current_exe() {
            let mut dir = exe_dir;
            for _ in 0..5 {
                dir.pop();
                let candidate = dir.join("SEARATES-API.json");
                if candidate.exists() {
                    return candidate.to_string_lossy().to_string();
                }
            }
            "SEARATES-API.json".to_string()
        } else {
            "SEARATES-API.json".to_string()
        }
    });

    tracing::info!("Starting SeaRates GraphQL Mock Server (pact-driven)...");
    tracing::info!("Using pact file: {}", pact_path);
    tracing::info!("Listening on port {}", port);

    let state = AppState::from_pact_file(&pact_path);
    let app = pact_mock_server::searates_graphql::build_app(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::info!("✅ Rates mock server ready at http://{}", addr);

    axum::serve(listener, app).await.unwrap();
}
