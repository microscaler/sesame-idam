//! SeaRates Terminal Tracking Mock Server
//!
//! Mocks the `GET /terminal` REST endpoint of the SeaRates Terminal Tracking API (v1.0).
//!
//! ## Usage
//!
//! ```bash
//! export PORT=8080
//! cargo run --bin searates-terminal
//! ```
//!
//! ## Endpoints
//!
//! - `GET /health` — Health check
//! - `GET /terminal` — Terminal tracking (requires `api_key`, `terminal_code`, `container_number` query params)
//!
//! ## Testing features (via headers)
//!
//! - `X-Rate-Limit: true` → 429 Too Many Requests
//! - `X-Service-Unavailable: true` → 503 Service Unavailable
//! - `X-Auth-Failure: 401` → 401 Unauthorized
//! - `X-Auth-Failure: 403` → 403 Forbidden

use axum::{
    extract::Query,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
};
use pact_mock_server::{
    auth_failure_middleware, health_check, logging_middleware, rate_limit_middleware,
    service_unavailable_middleware,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::{info, warn, Level};

// ============================================================================
// Query parameters
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct TerminalQueryParams {
    pub api_key: Option<String>,
    pub terminal_code: Option<String>,
    pub container_number: Option<String>,
}

// ============================================================================
// Seed data
// ============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TerminalEntry {
    request: TerminalRequest,
    response: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TerminalRequest {
    terminal_code: String,
    container_number: String,
}

fn terminal_entries() -> Vec<TerminalEntry> {
    vec![
        // --- Khalifa Port, Abu Dhabi (AEKHLADT) ---
        TerminalEntry {
            request: TerminalRequest {
                terminal_code: "AEKHLADT".to_string(),
                container_number: "SEKU5738515".to_string(),
            },
            response: r#"{
  "status_code": "OK",
  "metadata": {
    "request_parameters": {
      "terminal_code": "AEKHLADT",
      "container_number": "SEKU5738515"
    }
  },
  "data": {
    "terminal": {
      "name": "KHALIFA PORT CONTAINER TERMINAL",
      "operator": "ABU DHABI TERMINALS",
      "address": "Khalifa Port Container Terminal Building 70 Taweelah, Abu Dhabi, U.A.E.",
      "website": "https://www.adterminals.ae",
      "country_code": "AE",
      "locode": "AEKHL",
      "bic_code": null,
      "smdg_code": "AEKHLADT",
      "lat": 24.8075,
      "lng": 54.649444
    },
    "container": {
      "number": "SEKU5738515",
      "iso_code": "45G1",
      "size_type": "40' High Cube Dry",
      "status": "NOT_ON_TERMINAL",
      "updated_at": "2025-05-16 08:29:52",
      "events": [
        {
          "description": "GATE_IN",
          "event_code": "GTIN",
          "datetime": "2024-09-15 17:18:00",
          "is_actual": true,
          "is_empty": true,
          "transport_type": null,
          "vessel_name": null,
          "voyage": null
        },
        {
          "description": "GATE_OUT",
          "event_code": "GTOT",
          "datetime": "2024-12-12 03:28:00",
          "is_actual": true,
          "is_empty": true,
          "transport_type": "TRUCK",
          "vessel_name": null,
          "voyage": null
        },
        {
          "description": "GATE_IN",
          "event_code": "GTIN",
          "datetime": "2024-12-13 03:58:00",
          "is_actual": true,
          "is_empty": false,
          "transport_type": null,
          "vessel_name": null,
          "voyage": null
        },
        {
          "description": "LOAD",
          "event_code": "LOAD",
          "datetime": "2024-12-15 14:30:00",
          "is_actual": true,
          "is_empty": false,
          "transport_type": "VESSEL",
          "vessel_name": "EVER GIVEN",
          "voyage": "EG24"
        }
      ]
    }
  }
}"#
            .to_string(),
        },
        // --- Port of Shanghai, China (CNSHA01) ---
        TerminalEntry {
            request: TerminalRequest {
                terminal_code: "CNSHA01".to_string(),
                container_number: "TCLU1234567".to_string(),
            },
            response: r#"{
  "status_code": "OK",
  "metadata": {
    "request_parameters": {
      "terminal_code": "CNSHA01",
      "container_number": "TCLU1234567"
    }
  },
  "data": {
    "terminal": {
      "name": "SHANGHAI PORT CONTAINER TERMINAL",
      "operator": "SHANGHAI PORT GROUP",
      "address": "Yangshan Deep Water Port, Shanghai, P.R. China",
      "website": "https://www.shport.com",
      "country_code": "CN",
      "locode": "CNSHA",
      "bic_code": null,
      "smdg_code": "CNSHA01",
      "lat": 30.63,
      "lng": 122.15
    },
    "container": {
      "number": "TCLU1234567",
      "iso_code": "42G1",
      "size_type": "40' Standard Dry",
      "status": "ON_TERMINAL",
      "updated_at": "2025-06-20 10:15:00",
      "events": [
        {
          "description": "GATE_IN",
          "event_code": "GTIN",
          "datetime": "2025-06-18 09:00:00",
          "is_actual": true,
          "is_empty": false,
          "transport_type": "TRUCK",
          "vessel_name": null,
          "voyage": null
        },
        {
          "description": "DISCHARGE",
          "event_code": "DISCH",
          "datetime": "2025-06-20 06:30:00",
          "is_actual": true,
          "is_empty": false,
          "transport_type": "VESSEL",
          "vessel_name": "MAERSK SEATTLE",
          "voyage": "MS2501"
        }
      ]
    }
  }
}"#
            .to_string(),
        },
        // --- Rotterdam, Netherlands (NLRTM01) ---
        TerminalEntry {
            request: TerminalRequest {
                terminal_code: "NLRTM01".to_string(),
                container_number: "MSCU9876543".to_string(),
            },
            response: r#"{
  "status_code": "OK",
  "metadata": {
    "request_parameters": {
      "terminal_code": "NLRTM01",
      "container_number": "MSCU9876543"
    }
  },
  "data": {
    "terminal": {
      "name": "EUROPORT CONTAINER TERMINAL",
      "operator": "APM TERMINALS ROTTERDAM",
      "address": "Wilhelminakade 102, 3072 AP Rotterdam, Netherlands",
      "website": "https://www.apmterminals.com",
      "country_code": "NL",
      "locode": "NLRTM",
      "bic_code": null,
      "smdg_code": "NLRTM01",
      "lat": 51.9225,
      "lng": 4.47917
    },
    "container": {
      "number": "MSCU9876543",
      "iso_code": "42G1",
      "size_type": "40' Standard Dry",
      "status": "NOT_ON_TERMINAL",
      "updated_at": "2025-07-01 14:00:00",
      "events": [
        {
          "description": "GATE_OUT",
          "event_code": "GTOT",
          "datetime": "2025-06-30 11:00:00",
          "is_actual": true,
          "is_empty": false,
          "transport_type": "RAIL",
          "vessel_name": null,
          "voyage": null
        },
        {
          "description": "LOAD",
          "event_code": "LOAD",
          "datetime": "2025-06-28 16:45:00",
          "is_actual": true,
          "is_empty": false,
          "transport_type": "VESSEL",
          "vessel_name": "MSC GULSUN",
          "voyage": "MG0345"
        }
      ]
    }
  }
}"#
            .to_string(),
        },
    ]
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /terminal — Return terminal tracking data for the requested container
pub async fn get_terminal(
    Query(params): Query<TerminalQueryParams>,
    _headers: HeaderMap,
) -> Response {
    let terminal_code = match &params.terminal_code {
        Some(tc) => tc,
        None => {
            warn!("Missing required query param: terminal_code");
            return error_response(
                StatusCode::BAD_REQUEST,
                "Missing required query parameter: terminal_code",
                Some("MISSING_PARAMETER"),
            )
            .into_response();
        }
    };

    let container_number = match &params.container_number {
        Some(cn) => cn,
        None => {
            warn!("Missing required query param: container_number");
            return error_response(
                StatusCode::BAD_REQUEST,
                "Missing required query parameter: container_number",
                Some("MISSING_PARAMETER"),
            )
            .into_response();
        }
    };

    // Validate API key presence (empty string counts as missing)
    if params
        .api_key
        .as_ref()
        .map(|k| k.is_empty())
        .unwrap_or(true)
    {
        warn!("Missing or empty api_key query parameter");
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": {
                    "code": 401,
                    "message": "Unauthorized: Invalid token"
                }
            })),
        )
            .into_response();
    }

    info!(
        "  Terminal lookup: terminal_code={}, container_number={}",
        terminal_code, container_number
    );

    // Search for matching entry
    let entries = terminal_entries();
    if let Some(entry) = entries.iter().find(|e| {
        e.request.terminal_code == *terminal_code && e.request.container_number == *container_number
    }) {
        info!(
            "  Found terminal data: {} / {}",
            terminal_code, container_number
        );
        return (
            StatusCode::OK,
            Json(serde_json::from_str::<Value>(&entry.response).unwrap_or(json!({}))),
        )
            .into_response();
    }

    warn!(
        "  No terminal data found for terminal_code={}, container_number={}",
        terminal_code, container_number
    );
    error_response(
        StatusCode::NOT_FOUND,
        format!(
            "No tracking data found for terminal {} and container {}",
            terminal_code, container_number
        ),
        Some("NOT_FOUND"),
    )
    .into_response()
}

// ============================================================================
// Helpers
// ============================================================================

fn error_response(
    status: StatusCode,
    message: impl Into<String>,
    code: Option<&str>,
) -> Json<Value> {
    let mut map = serde_json::Map::new();
    map.insert("code".to_string(), Value::Number(status.as_u16().into()));
    map.insert("message".to_string(), Value::String(message.into()));
    if let Some(c) = code {
        map.insert("error_code".to_string(), Value::String(c.to_string()));
    }
    let mut root = serde_json::Map::new();
    root.insert("error".to_string(), Value::Object(map));
    Json(Value::Object(root))
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid u16");

    info!("Starting SeaRates Terminal Tracking Mock Server...");
    info!("Listening on port {}", port);

    let app = Router::new()
        .route("/", get(health_check))
        .route("/health", get(health_check))
        .route("/terminal", get(get_terminal))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(axum::middleware::from_fn(auth_failure_middleware))
                .layer(axum::middleware::from_fn(service_unavailable_middleware))
                .layer(axum::middleware::from_fn(rate_limit_middleware))
                .layer(axum::middleware::from_fn(logging_middleware)),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("✅ Terminal mock server ready at http://{}", addr);

    axum::serve(listener, app).await.unwrap();
}
