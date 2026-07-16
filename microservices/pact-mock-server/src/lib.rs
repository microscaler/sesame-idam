//! Shared library for Pact Mock Servers
//!
//! Provides common functionality for Searates mock servers:
//! - Contract loading from Pact broker
//! - Request logging middleware
//! - Health check endpoints
//! - App state management
//! - API path definitions (source of truth for Searates API paths)
//!
//! ## Quick Start
//!
//! ```rust
//! use pact_mock_server::prelude::*;
//! ```
//!
//! This brings commonly used types and functions into scope.
pub mod prelude;
pub mod searates_graphql;
pub mod sesame_idam_broker;
use axum::{
    extract::Request,
    http::{HeaderValue, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{info, warn};

/// Application state shared across all mock servers
///
/// (GcpSecretStore, AwsSecretStore, AzureSecretStore) instead of this generic AppState.
/// This is kept for backward compatibility and shared functionality.
#[derive(Clone, Debug)]
pub struct AppState {
    pub contracts: Arc<RwLock<HashMap<String, Value>>>,
}

impl AppState {
    pub fn new(contracts: HashMap<String, Value>) -> Self {
        Self {
            contracts: Arc::new(RwLock::new(contracts)),
        }
    }
}

/// Request logging middleware
/// Logs all incoming requests with method, path, client IP, response status, and duration
pub async fn logging_middleware(request: Request, next: Next) -> axum::response::Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let start = std::time::Instant::now();

    // Log request
    info!(
        "→ {} {} [client: {}]",
        method,
        path,
        request
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
    );

    // Log request body for POST/PUT/PATCH
    if matches!(method, Method::POST | Method::PUT | Method::PATCH) {
        if let Some(content_type) = request.headers().get("content-type") {
            if content_type
                .to_str()
                .unwrap_or("")
                .contains("application/json")
            {
                // Note: Body is consumed by the handler, so we can't log it here
                // The handler will log it if needed
            }
        }
    }

    let response = next.run(request).await;
    let duration = start.elapsed();
    let status = response.status();

    info!(
        "← {} {} [{}] [{:.3}s]",
        method,
        path,
        status.as_u16(),
        duration.as_secs_f64()
    );

    response
}

/// Health check endpoint
/// Returns a simple JSON response indicating the service is healthy
pub async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "pact-mock-server"
    }))
}

/// Rate limiting middleware
/// Returns 429 Too Many Requests if request includes X-Rate-Limit header
///
/// This allows testing individual requests without affecting the entire server.
/// To trigger a 429 response, include the header: X-Rate-Limit: true
///
/// Optional header: X-Rate-Limit-Retry-After: <seconds> (default: 60)
/// This sets the Retry-After header value in the response.
pub async fn rate_limit_middleware(request: Request, next: Next) -> Response {
    // Check for X-Rate-Limit header
    if let Some(header_value) = request.headers().get("x-rate-limit") {
        if let Ok(header_str) = header_value.to_str() {
            if header_str.to_lowercase() == "true" {
                // Get optional retry-after value from header
                let retry_after = request
                    .headers()
                    .get("x-rate-limit-retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(60);

                warn!(
                    "Rate limit header detected - returning 429 with retry-after: {}s",
                    retry_after
                );

                let mut response = (
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(json!({
                        "error": {
                            "code": 429,
                            "message": "Rate limit exceeded",
                            "retry_after": retry_after
                        }
                    })),
                )
                    .into_response();

                // Add Retry-After header
                if let Ok(header_value) = HeaderValue::from_str(&retry_after.to_string()) {
                    response.headers_mut().insert("retry-after", header_value);
                }

                return response;
            }
        }
    }

    next.run(request).await
}

/// Service unavailable middleware
/// Returns 503 Service Unavailable if request includes X-Service-Unavailable header
///
/// This allows testing individual requests without affecting the entire server.
/// To trigger a 503 response, include the header: X-Service-Unavailable: true
pub async fn service_unavailable_middleware(request: Request, next: Next) -> Response {
    // Skip health checks
    if request.uri().path() == "/health" || request.uri().path() == "/" {
        return next.run(request).await;
    }

    // Check for X-Service-Unavailable header
    if let Some(header_value) = request.headers().get("x-service-unavailable") {
        if let Ok(header_str) = header_value.to_str() {
            if header_str.to_lowercase() == "true" {
                warn!("Service unavailable header detected - returning 503");
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(json!({
                        "error": {
                            "code": 503,
                            "message": "Service temporarily unavailable"
                        }
                    })),
                )
                    .into_response();
            }
        }
    }

    next.run(request).await
}

/// Authentication failure middleware
/// Returns 401 Unauthorized or 403 Forbidden if request includes authentication failure headers
///
/// This allows testing individual requests without affecting the entire server.
/// To trigger a 401 response, include the header: X-Auth-Failure: 401
/// To trigger a 403 response, include the header: X-Auth-Failure: 403
pub async fn auth_failure_middleware(request: Request, next: Next) -> Response {
    // Skip health checks
    if request.uri().path() == "/health" || request.uri().path() == "/" {
        return next.run(request).await;
    }

    // Check for X-Auth-Failure header
    if let Some(header_value) = request.headers().get("x-auth-failure") {
        if let Ok(header_str) = header_value.to_str() {
            let status = match header_str {
                "401" | "unauthorized" => Some(StatusCode::UNAUTHORIZED),
                "403" | "forbidden" => Some(StatusCode::FORBIDDEN),
                _ => None,
            };

            if let Some(status_code) = status {
                warn!("Auth failure header detected - returning {}", status_code);
                if request.uri().path() == "/graphql" {
                    let message = if status_code == StatusCode::UNAUTHORIZED {
                        "Incorrect token, invalid format"
                    } else {
                        "Forbidden: Insufficient permissions"
                    };
                    return (
                        status_code,
                        Json(json!({
                            "errors": [{
                                "message": message,
                                "status_code": if status_code == StatusCode::UNAUTHORIZED { "AUTHORIZATION_ERROR" } else { "FORBIDDEN" },
                                "error_code": status_code.as_u16()
                            }]
                        })),
                    )
                        .into_response();
                }
                return (
                    status_code,
                    Json(json!({
                        "error": {
                            "code": status_code.as_u16(),
                            "message": if status_code == StatusCode::UNAUTHORIZED {
                                "Unauthorized: Invalid or missing authentication credentials"
                            } else {
                                "Forbidden: Insufficient permissions"
                            }
                        }
                    })),
                )
                    .into_response();
            }
        }
    }

    next.run(request).await
}

/// Wait for Pact broker to be ready and pacts to be published
/// This ensures the broker is accessible and contracts are available before starting the mock server
pub async fn wait_for_broker_and_pacts(
    broker_url: &str,
    username: &str,
    password: &str,
    provider: &str,
    consumer: &str,
    max_wait_seconds: u64,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let start_time = std::time::Instant::now();
    let mut attempt = 0;
    let max_attempts = max_wait_seconds / 2; // Check every 2 seconds

    info!("Waiting for Pact broker to be ready and pacts to be published...");
    info!(
        "Broker: {}, Provider: {}, Consumer: {}",
        broker_url, provider, consumer
    );

    loop {
        attempt += 1;

        // Check if we've exceeded max wait time
        if start_time.elapsed().as_secs() >= max_wait_seconds {
            return Err(format!(
                "Timeout waiting for broker and pacts after {} seconds",
                max_wait_seconds
            ));
        }

        // First, check if broker is accessible
        let heartbeat_url = format!("{}/diagnostic/status/heartbeat", broker_url);
        match client.get(&heartbeat_url).send().await {
            Ok(response) if response.status().is_success() => {
                // Broker is ready, now check for pacts
                let pact_url = format!(
                    "{}/pacts/provider/{}/consumer/{}/latest",
                    broker_url, provider, consumer
                );

                match client
                    .get(&pact_url)
                    .basic_auth(username, Some(password))
                    .send()
                    .await
                {
                    Ok(response) if response.status().is_success() => {
                        info!("✅ Broker is ready and pacts are published!");
                        return Ok(());
                    }
                    Ok(response) if response.status() == 404 => {
                        if attempt % 5 == 0 {
                            info!(
                                "Broker is ready, waiting for pacts to be published... (attempt {}/{})",
                                attempt, max_attempts
                            );
                        }
                    }
                    Ok(response) => {
                        warn!(
                            "Unexpected status when checking for pacts: {}",
                            response.status()
                        );
                    }
                    Err(e) => {
                        if attempt % 5 == 0 {
                            warn!("Error checking for pacts: {} (will retry)", e);
                        }
                    }
                }
            }
            Ok(response) => {
                if attempt % 5 == 0 {
                    warn!("Broker returned status {} (will retry)", response.status());
                }
            }
            Err(e) => {
                if attempt % 5 == 0 {
                    warn!("Broker not yet accessible: {} (will retry)", e);
                }
            }
        }

        // Wait before next attempt
        sleep(Duration::from_secs(2)).await;
    }
}

/// Wait for manager to be ready and for a specific provider's pact to be published
///
/// Polls the manager's `/ready` endpoint and checks if the provider's pact is in the
/// `published_providers` list. This is more reliable than checking the broker directly
/// because the manager tracks which pacts have been successfully published.
pub async fn wait_for_manager_ready(
    manager_url: &str,
    provider: &str,
    max_wait_seconds: u64,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let start_time = std::time::Instant::now();
    let mut attempt = 0;
    let max_attempts = max_wait_seconds / 2; // Check every 2 seconds

    info!("Waiting for manager to be ready and pact to be published...");
    info!("Manager: {}, Provider: {}", manager_url, provider);

    loop {
        attempt += 1;

        // Check if we've exceeded max wait time
        if start_time.elapsed().as_secs() >= max_wait_seconds {
            return Err(format!(
                "Timeout waiting for manager and pact after {} seconds",
                max_wait_seconds
            ));
        }

        // Check manager's /ready endpoint
        let ready_url = format!("{}/ready", manager_url);
        match client
            .get(&ready_url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                // Parse the JSON response
                match response.json::<serde_json::Value>().await {
                    Ok(json) => {
                        let status = json
                            .get("status")
                            .and_then(|s| s.as_str())
                            .unwrap_or("unknown");
                        let broker_healthy = json
                            .get("broker_healthy")
                            .and_then(|b| b.as_bool())
                            .unwrap_or(false);
                        let pacts_published = json
                            .get("pacts_published")
                            .and_then(|p| p.as_bool())
                            .unwrap_or(false);
                        let published_providers = json
                            .get("published_providers")
                            .and_then(|p| p.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect::<Vec<String>>()
                            })
                            .unwrap_or_default();

                        // Check if manager is ready (broker healthy AND pacts published)
                        // /ready endpoint requires both broker healthy and pacts published
                        if status == "ready" && broker_healthy && pacts_published {
                            // Check if our provider is in the published list
                            // Provider names are stored exactly as they appear in the ConfigMap
                            // (e.g., "AWS-Secrets-Manager", "GCP-Secret-Manager", "Azure-Key-Vault")
                            let provider_found = published_providers
                                .iter()
                                .any(|p| p.eq_ignore_ascii_case(provider));

                            if provider_found {
                                info!(
                                    "✅ Manager is ready and pact for provider '{}' is published!",
                                    provider
                                );
                                info!("   Published providers: {:?}", published_providers);
                                return Ok(());
                            } else {
                                if attempt % 5 == 0 {
                                    info!(
                                        "Manager is ready, but provider '{}' not yet published... (attempt {}/{})",
                                        provider, attempt, max_attempts
                                    );
                                    info!(
                                        "   Published providers so far: {:?}",
                                        published_providers
                                    );
                                }
                            }
                        } else {
                            if attempt % 5 == 0 {
                                info!(
                                    "Manager not yet ready: status={}, broker_healthy={}, pacts_published={} (attempt {}/{})",
                                    status, broker_healthy, pacts_published, attempt, max_attempts
                                );
                            }
                        }
                    }
                    Err(e) => {
                        if attempt % 5 == 0 {
                            warn!("Error parsing manager response: {} (will retry)", e);
                        }
                    }
                }
            }
            Ok(response) => {
                if attempt % 5 == 0 {
                    warn!("Manager returned status {} (will retry)", response.status());
                }
            }
            Err(e) => {
                if attempt % 5 == 0 {
                    warn!("Manager not yet accessible: {} (will retry)", e);
                }
            }
        }

        // Wait before next attempt
        sleep(Duration::from_secs(2)).await;
    }
}

/// Load contracts from Pact broker
/// Fetches the latest contracts for a given provider and consumer
pub async fn load_contracts_from_broker(
    broker_url: &str,
    username: &str,
    password: &str,
    provider: &str,
    consumer: &str,
) -> HashMap<String, Value> {
    let url = format!(
        "{}/pacts/provider/{}/consumer/{}/latest",
        broker_url, provider, consumer
    );

    let client = reqwest::Client::new();
    match client
        .get(&url)
        .basic_auth(username, Some(password))
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => match response.json::<Value>().await {
            Ok(contracts) => {
                info!("✅ Loaded contracts from broker");
                let mut map = HashMap::new();
                map.insert("contracts".to_string(), contracts);
                map
            }
            Err(e) => {
                warn!("Failed to parse contracts: {}", e);
                HashMap::new()
            }
        },
        Ok(response) => {
            warn!(
                "Could not load contracts from broker: {}",
                response.status()
            );
            HashMap::new()
        }
        Err(e) => {
            warn!("Could not connect to broker: {}", e);
            HashMap::new()
        }
    }
}
