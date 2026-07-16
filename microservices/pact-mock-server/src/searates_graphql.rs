//! SeaRates GraphQL mock module — shared between the binary and tests.
//!
//! Pact-driven mock for the SeaRates Logistics Explorer / Get Rates GraphQL API (v3.0).

pub use crate::auth_failure_middleware;
pub use crate::health_check;
pub use crate::logging_middleware;
pub use crate::rate_limit_middleware;
pub use crate::service_unavailable_middleware;

use axum::{
    extract::{Request, State},
    http::{header, Method, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

// Embed the pact file at compile time as a fallback
const EMBEDDED_PACT: &str = include_str!("../SEARATES-API.json");

// ============================================================================
// GraphQL request types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct GraphqlRequest {
    pub query: String,
    #[serde(default)]
    pub variables: Option<Value>,
}

/// Extracted query parameters from a GraphQL request
#[derive(Debug, Default)]
pub struct RateParams {
    pub shipping_type: Option<String>,
    pub container: Option<String>,
    pub coordinates_from: Option<Vec<f64>>,
    pub coordinates_to: Option<Vec<f64>>,
    pub point_id_from: Option<String>,
    pub point_id_to: Option<String>,
    pub date: Option<String>,
}

/// A loaded pact interaction for matching and response
#[derive(Debug, Clone)]
pub struct PactInteraction {
    pub description: String,
    pub status: StatusCode,
    pub response_body: Value,
    /// Which shipping types this interaction handles (for successful responses)
    pub shipping_types: HashSet<String>,
}

impl PactInteraction {
    /// Load pact interactions from the SEARATES-API.json file
    fn from_pact_file(path: &str) -> Vec<Self> {
        let content = std::fs::read_to_string(path).unwrap_or_else(|e| {
            warn!("Failed to read pact file {path}: {e} (using embedded fallback)");
            EMBEDDED_PACT.to_string()
        });

        let pact: Value =
            serde_json::from_str(&content).expect("Failed to parse SEARATES-API.json");

        let interactions = pact
            .get("interactions")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut results = Vec::new();

        for inter in &interactions {
            let description = inter
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("unnamed")
                .to_string();

            let status = inter
                .get("response")
                .and_then(|v| v.get("status"))
                .and_then(|v| v.as_u64())
                .unwrap_or(200);
            let status = match status {
                200 => StatusCode::OK,
                400 => StatusCode::BAD_REQUEST,
                401 => StatusCode::UNAUTHORIZED,
                404 => StatusCode::NOT_FOUND,
                500 => StatusCode::INTERNAL_SERVER_ERROR,
                _ => StatusCode::from_u16(status as u16).unwrap_or(StatusCode::OK),
            };

            let json_body = inter
                .get("response")
                .and_then(|v| v.get("jsonBody"))
                .cloned()
                .unwrap_or(json!({}));

            // For successful responses, determine shipping type from the points data
            let mut shipping_types = HashSet::new();

            if status.is_success() {
                if let Some(rates) = json_body.get("rates").and_then(|v| v.as_array()) {
                    for rate in rates {
                        if let Some(points) = rate.get("points").and_then(|v| v.as_array()) {
                            for point in points {
                                if let Some(st) = point.get("shippingType").and_then(|v| v.as_str())
                                {
                                    shipping_types.insert(st.to_string());
                                }
                            }
                        }
                    }
                }
            }

            results.push(PactInteraction {
                description,
                status,
                response_body: json_body,
                shipping_types,
            });
        }

        results
    }
}

/// Application state with loaded pact interactions
#[derive(Clone)]
pub struct AppState {
    pub interactions: Arc<RwLock<Vec<PactInteraction>>>,
}

impl AppState {
    pub fn from_pact_file(path: &str) -> Self {
        let interactions = PactInteraction::from_pact_file(path);
        info!(
            "Loaded {} pact interactions for SeaRates",
            interactions.len()
        );
        for (i, inter) in interactions.iter().enumerate() {
            let types: Vec<_> = inter.shipping_types.iter().collect();
            info!(
                "  [{i}] {} (status={}, types={:?})",
                inter.description, inter.status, types
            );
        }
        Self {
            interactions: Arc::new(RwLock::new(interactions)),
        }
    }
}

// ============================================================================
// GraphQL query parser
// ============================================================================

fn extract_string_value(query: &str, field: &str) -> Option<String> {
    let pattern = format!("{field}: \"");
    if let Some(start) = query.find(&pattern) {
        let after_key = &query[start + pattern.len()..];
        if let Some(end) = after_key.find('"') {
            return Some(after_key[..end].to_string());
        }
    }

    let pattern = format!("{field}:");
    if let Some(start) = query.find(&pattern) {
        let after_key = query[start + pattern.len()..].trim_start();
        let end = after_key
            .find(|c: char| {
                c == ',' || c == ')' || c == '}' || c == '(' || c == '\n' || c.is_whitespace()
            })
            .unwrap_or(after_key.len());
        let val = after_key[..end].trim().to_string();
        if !val.is_empty() && val != "null" {
            return Some(val);
        }
    }

    None
}

fn extract_float_array(query: &str, field: &str) -> Option<Vec<f64>> {
    let pattern = format!("{field}: [");
    if let Some(start) = query.find(&pattern) {
        let after_key = &query[start + pattern.len()..];
        if let Some(end) = after_key.find(']') {
            let arr_str = &after_key[..end];
            return Some(
                arr_str
                    .split(',')
                    .filter_map(|s| s.trim().parse::<f64>().ok())
                    .collect(),
            );
        }
    }
    None
}

fn extract_string_array(query: &str, field: &str) -> Option<Vec<String>> {
    let pattern = format!("{field}: [");
    if let Some(start) = query.find(&pattern) {
        let after_key = &query[start + pattern.len()..];
        if let Some(end) = after_key.find(']') {
            let arr_str = &after_key[..end];
            return Some(
                arr_str
                    .split(',')
                    .filter_map(|s| {
                        let trimmed = s.trim().trim_matches('"').to_string();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed)
                        }
                    })
                    .collect(),
            );
        }
    }
    None
}

fn extract_rate_params(query: &str) -> RateParams {
    let mut params = RateParams::default();

    if let Some(val) = extract_string_value(query, "shippingType") {
        params.shipping_type = Some(val);
    }
    if params.shipping_type.is_none() {
        if let Some(val) = extract_string_value(query, "shippingTypes") {
            params.shipping_type = Some(val);
        }
    }
    if params.shipping_type.is_none() {
        if let Some(arr) = extract_string_array(query, "shippingType") {
            if let Some(first) = arr.first() {
                params.shipping_type = Some(first.clone());
            }
        }
    }
    if params.shipping_type.is_none() {
        if let Some(arr) = extract_string_array(query, "shippingTypes") {
            if let Some(first) = arr.first() {
                params.shipping_type = Some(first.clone());
            }
        }
    }
    if let Some(val) = extract_string_value(query, "container") {
        params.container = Some(val);
    }
    if let Some(val) = extract_string_value(query, "date") {
        params.date = Some(val);
    }
    if let Some(val) = extract_string_value(query, "pointIdFrom") {
        params.point_id_from = Some(val);
    }
    if let Some(val) = extract_string_value(query, "pointIdTo") {
        params.point_id_to = Some(val);
    }
    if let Some(arr) = extract_float_array(query, "coordinatesFrom") {
        params.coordinates_from = Some(arr);
    }
    if let Some(arr) = extract_float_array(query, "coordinatesTo") {
        params.coordinates_to = Some(arr);
    }

    params
}

fn apply_graphql_variables(params: &mut RateParams, variables: &Value) {
    let params_vars = variables.get("params").unwrap_or(variables);

    if let Some(shipping_type) = params_vars
        .get("shippingType")
        .or_else(|| params_vars.get("shipping_type"))
        .and_then(|v| v.as_str())
    {
        params.shipping_type = Some(shipping_type.to_string());
    }
    if let Some(shipping_types) = params_vars
        .get("shippingTypes")
        .or_else(|| params_vars.get("shipping_types"))
        .and_then(|v| v.as_array())
    {
        for item in shipping_types {
            if let Some(st) = item.as_str() {
                let valid_types = ["FCL", "LCL", "fcl", "lcl"];
                let st_upper = st.to_uppercase();
                if !valid_types.contains(&st_upper.as_str()) {
                    params.shipping_type = Some(st.to_string());
                    return;
                }
                if params.shipping_type.is_none() {
                    params.shipping_type = Some(st.to_string());
                }
            }
        }
    }
    if let Some(point_id_from) = params_vars
        .get("pointIdFrom")
        .or_else(|| params_vars.get("point_id_from"))
        .and_then(|v| v.as_str())
    {
        params.point_id_from = Some(point_id_from.to_string());
    }
    if let Some(point_id_to) = params_vars
        .get("pointIdTo")
        .or_else(|| params_vars.get("point_id_to"))
        .and_then(|v| v.as_str())
    {
        params.point_id_to = Some(point_id_to.to_string());
    }
    if let Some(date) = params_vars.get("date").and_then(|v| v.as_str()) {
        params.date = Some(date.to_string());
    }
    if let Some(container) = params_vars.get("container").and_then(|v| v.as_str()) {
        params.container = Some(container.to_string());
    }
    if let Some(coords_from) = params_vars
        .get("coordinatesFrom")
        .or_else(|| params_vars.get("coordinates_from"))
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter()
                .filter_map(|v| v.as_f64())
                .collect::<Vec<f64>>()
                .into()
        })
    {
        params.coordinates_from = Some(coords_from);
    }
    if let Some(coords_to) = params_vars
        .get("coordinatesTo")
        .or_else(|| params_vars.get("coordinates_to"))
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter()
                .filter_map(|v| v.as_f64())
                .collect::<Vec<f64>>()
                .into()
        })
    {
        params.coordinates_to = Some(coords_to);
    }
}

async fn graphql_handler(State(state): State<AppState>, request: Request) -> Response {
    if let Some(hv) = request.headers().get("x-auth-failure") {
        if let Ok(s) = hv.to_str() {
            match s {
                "401" | "unauthorized" => {
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(json!({
                            "errors": [{"message": "Incorrect token, invalid format", "status_code": "AUTHORIZATION_ERROR", "error_code": 401}]
                        })),
                    )
                        .into_response();
                }
                "403" | "forbidden" => {
                    return (
                        StatusCode::FORBIDDEN,
                        Json(json!({
                            "errors": [{"message": "Forbidden: Insufficient permissions"}]
                        })),
                    )
                        .into_response();
                }
                _ => {}
            }
        }
    }

    if let Some(hv) = request.headers().get("x-service-unavailable") {
        if let Ok(s) = hv.to_str() {
            if s.to_lowercase() == "true" {
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

    if let Some(hv) = request.headers().get("x-rate-limit") {
        if let Ok(s) = hv.to_str() {
            if s.to_lowercase() == "true" {
                let retry_after = request
                    .headers()
                    .get("x-rate-limit-retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(60);

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
                if let Ok(hv) = header::HeaderValue::from_str(&retry_after.to_string()) {
                    response.headers_mut().insert(header::RETRY_AFTER, hv);
                }
                return response;
            }
        }
    }

    if *request.method() != Method::POST {
        return (
            StatusCode::METHOD_NOT_ALLOWED,
            Json(json!({
                "errors": [{"message": "Method not allowed", "extensions": {"code": "METHOD_NOT_ALLOWED"}}]
            })),
        )
            .into_response();
    }

    let body_bytes = match axum::body::to_bytes(request.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            warn!("Failed to read request body: {e}");
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "errors": [{"message": "Failed to read request body", "extensions": {"code": "BAD_REQUEST"}}]
                })),
            )
                .into_response();
        }
    };

    if body_bytes.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "errors": [{"message": "Request body is required", "extensions": {"code": "BAD_REQUEST"}}]
            })),
        )
            .into_response();
    }

    let (query_str, variables): (String, Option<Value>) = match serde_json::from_slice::<
        GraphqlRequest,
    >(&body_bytes)
    {
        Ok(req) => (req.query, req.variables),
        Err(_) => match serde_json::from_slice::<String>(&body_bytes) {
            Ok(s) => (s, None),
            Err(_) => {
                warn!("  Invalid JSON request body");
                return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "errors": [{"message": "Invalid JSON. Expected GraphQL query.", "extensions": {"code": "BAD_REQUEST"}}]
                        })),
                    )
                        .into_response();
            }
        },
    };

    let mut params = extract_rate_params(&query_str);

    if let Some(ref vars) = variables {
        apply_graphql_variables(&mut params, vars);
    }

    info!(
        "GraphQL query: shipping_type={:?}, from={:?}, to={:?}",
        params.shipping_type, params.point_id_from, params.point_id_to
    );

    let interactions = state.interactions.read().await;

    // Check for invalid shipping type
    if let Some(ref st) = params.shipping_type {
        let valid_types = ["FCL", "LCL", "fcl", "lcl"];
        let st_upper = st.to_uppercase();
        if !valid_types.contains(&st_upper.as_str()) {
            for inter in interactions.iter() {
                if inter.status == StatusCode::BAD_REQUEST
                    && inter.description.contains("INVALID_TYPE")
                {
                    info!("  Matched invalid type interaction: {}", inter.description);
                    return (inter.status, Json(inter.response_body.clone())).into_response();
                }
            }
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "errors": [{
                        "message": "INVALID_TYPE: shipping type must be FCL or LCL",
                        "extensions": {"code": "INVALID_TYPE"}
                    }]
                })),
            )
                .into_response();
        }
    }

    // Check for "place not found" errors
    if let (Some(ref pid_from), Some(ref pid_to)) = (&params.point_id_from, &params.point_id_to) {
        let known: HashSet<&str> = [
            "UAODS", "JPTYO", "CNSHA", "USLAX", "NLRTM", "DEHAM", "CNYTN",
        ]
        .into_iter()
        .collect();

        if !known.contains(pid_from.as_str()) || !known.contains(pid_to.as_str()) {
            for inter in interactions.iter() {
                if inter.status == StatusCode::BAD_REQUEST
                    && inter.description.contains("Place was not found")
                {
                    info!(
                        "  Matched place not found interaction: {}",
                        inter.description
                    );
                    return (inter.status, Json(inter.response_body.clone())).into_response();
                }
            }
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "errors": [{
                        "message": "Place was not found",
                        "extensions": {"code": "PLACE_NOT_FOUND"}
                    }]
                })),
            )
                .into_response();
        }
    }

    // Match success responses
    if let Some(ref st) = params.shipping_type {
        let st_upper = st.to_uppercase();
        for inter in interactions.iter() {
            if inter.status.is_success() {
                if inter.shipping_types.contains(&st_upper)
                    || inter.shipping_types.contains(&st.to_lowercase())
                {
                    info!("  Matched interaction: {} (type={})", inter.description, st);
                    let mut response = inter.response_body.clone();
                    if !response.get("data").is_some() {
                        response = json!({"data": response});
                    }
                    return (inter.status, Json(response)).into_response();
                }
            }
        }
    }

    warn!("  No matching interaction found");
    (StatusCode::OK, Json(json!({"data": {"rates": []}}))).into_response()
}

// ============================================================================
// App builder (exposed for testing)
// ============================================================================

/// Build the axum router with all routes and middleware layers.
pub fn build_app(state: AppState) -> Router {
    Router::new()
        .route("/", get(health_check))
        .route("/health", get(health_check))
        .route("/graphql", post(graphql_handler))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(axum::middleware::from_fn(auth_failure_middleware))
                .layer(axum::middleware::from_fn(service_unavailable_middleware))
                .layer(axum::middleware::from_fn(rate_limit_middleware))
                .layer(axum::middleware::from_fn(logging_middleware)),
        )
        .with_state(state)
}
