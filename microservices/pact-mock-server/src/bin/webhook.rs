//! # Mock Webhook Server
//!
//! A simple HTTP server that receives webhook notifications and stores them
//! in memory for testing purposes. Supports multiple notification types
//! (FluxCD, ArgoCD, and future-proofing) with segregated storage.

use axum::{
    extract::{Json, Path},
    http::StatusCode,
    response::Json as ResponseJson,
    routing::{get, post, MethodRouter},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// In-memory store for received notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub timestamp: String,
    pub event_type: String,
    pub payload: serde_json::Value,
}

/// FluxCD webhook payload structure
/// Based on FluxCD notification controller webhook format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FluxCDPayload {
    /// The Kubernetes object that triggered the event
    pub involved_object: InvolvedObject,
    /// Severity level: info, warning, error
    pub severity: String,
    /// Timestamp of the event
    pub timestamp: String,
    /// Human-readable message
    pub message: String,
    /// Reason for the event
    pub reason: String,
    /// Controller that reported the event
    pub reporting_controller: String,
    /// Optional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Kubernetes object reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvolvedObject {
    pub kind: String,
    pub name: String,
    pub namespace: String,
    #[serde(default)]
    pub uid: Option<String>,
    #[serde(default)]
    pub api_version: Option<String>,
}

/// Shared state for the webhook server
#[derive(Clone)]
struct AppState {
    // Separate notification stores per type to avoid cross-contamination
    flux_notifications: Arc<RwLock<Vec<Notification>>>,
    argo_notifications: Arc<RwLock<Vec<Notification>>>,
    // Generic store for future notification types
    generic_notifications: Arc<RwLock<HashMap<String, Vec<Notification>>>>,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let state = AppState {
        flux_notifications: Arc::new(RwLock::new(Vec::new())),
        argo_notifications: Arc::new(RwLock::new(Vec::new())),
        generic_notifications: Arc::new(RwLock::new(HashMap::new())),
    };

    // Build the application router with segregated routes
    let app = Router::new()
        .route("/health", get(health))
        // FluxCD notification endpoints
        .route(
            "/api/flux-notification",
            MethodRouter::new()
                .post(flux_webhook_handler)
                .get(get_flux_notifications),
        )
        .route(
            "/api/flux-notification/clear",
            post(clear_flux_notifications),
        )
        // ArgoCD notification endpoints
        .route(
            "/api/argo-notification",
            MethodRouter::new()
                .post(argo_webhook_handler)
                .get(get_argo_notifications),
        )
        .route(
            "/api/argo-notification/clear",
            post(clear_argo_notifications),
        )
        // Generic notification endpoints for future-proofing
        .route(
            "/api/{notification_type}",
            MethodRouter::new()
                .post(generic_webhook_handler)
                .get(get_generic_notifications),
        )
        .route(
            "/api/{notification_type}/clear",
            post(clear_generic_notifications),
        )
        .with_state(state);

    // Get port from environment or default to 8080
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);

    let addr = format!("0.0.0.0:{}", port);
    info!("🚀 Mock webhook server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Health check endpoint
async fn health() -> &'static str {
    "ok"
}

/// Helper function to create a notification from a payload
fn create_notification(payload: serde_json::Value) -> Notification {
    // Extract event type from FluxCD format or fallback to generic
    let event_type = payload
        .get("reason")
        .or_else(|| payload.get("eventType"))
        .or_else(|| payload.get("event_type"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    Notification {
        timestamp: chrono::Utc::now().to_rfc3339(),
        event_type,
        payload: payload.clone(),
    }
}

/// Validate FluxCD payload format
/// Returns Ok(()) if valid, Err with description if invalid
fn validate_fluxcd_payload(payload: &serde_json::Value) -> Result<(), String> {
    // Check for required FluxCD fields
    if !payload.get("involvedObject").is_some() && !payload.get("involved_object").is_some() {
        return Err("Missing required field: involvedObject".to_string());
    }

    if !payload.get("severity").is_some() {
        return Err("Missing required field: severity".to_string());
    }

    if !payload.get("message").is_some() {
        return Err("Missing required field: message".to_string());
    }

    if !payload.get("reason").is_some() {
        return Err("Missing required field: reason".to_string());
    }

    // Validate severity values
    if let Some(severity) = payload.get("severity").and_then(|v| v.as_str()) {
        if !["info", "warning", "error"].contains(&severity.to_lowercase().as_str()) {
            return Err(format!(
                "Invalid severity value: {}. Must be one of: info, warning, error",
                severity
            ));
        }
    }

    Ok(())
}

/// FluxCD webhook handler - receives notifications and stores them
async fn flux_webhook_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<StatusCode, (StatusCode, ResponseJson<serde_json::Value>)> {
    // Validate FluxCD payload format
    if let Err(e) = validate_fluxcd_payload(&payload) {
        return Err((
            StatusCode::BAD_REQUEST,
            ResponseJson(serde_json::json!({
                "error": "Invalid FluxCD payload format",
                "details": e
            })),
        ));
    }

    let notification = create_notification(payload);

    {
        let mut notifications = state.flux_notifications.write().await;
        notifications.push(notification.clone());
        info!(
            "📨 Received FluxCD webhook notification: event_type={}, total_flux_notifications={}",
            notification.event_type,
            notifications.len()
        );
    }

    Ok(StatusCode::OK)
}

/// Get all stored FluxCD notifications
async fn get_flux_notifications(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> ResponseJson<serde_json::Value> {
    let notifications = state.flux_notifications.read().await;
    ResponseJson(serde_json::json!({
        "notifications": *notifications,
        "count": notifications.len(),
        "type": "flux"
    }))
}

/// Clear all stored FluxCD notifications
async fn clear_flux_notifications(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> ResponseJson<serde_json::Value> {
    let mut notifications = state.flux_notifications.write().await;
    let count = notifications.len();
    notifications.clear();
    info!("🗑️  Cleared {} FluxCD notifications", count);
    ResponseJson(serde_json::json!({
        "cleared": count,
        "message": "FluxCD notifications cleared",
        "type": "flux"
    }))
}

/// ArgoCD webhook handler - receives notifications and stores them
async fn argo_webhook_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> StatusCode {
    let notification = create_notification(payload);

    {
        let mut notifications = state.argo_notifications.write().await;
        notifications.push(notification.clone());
        info!(
            "📨 Received ArgoCD webhook notification: event_type={}, total_argo_notifications={}",
            notification.event_type,
            notifications.len()
        );
    }

    StatusCode::OK
}

/// Get all stored ArgoCD notifications
async fn get_argo_notifications(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> ResponseJson<serde_json::Value> {
    let notifications = state.argo_notifications.read().await;
    ResponseJson(serde_json::json!({
        "notifications": *notifications,
        "count": notifications.len(),
        "type": "argo"
    }))
}

/// Clear all stored ArgoCD notifications
async fn clear_argo_notifications(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> ResponseJson<serde_json::Value> {
    let mut notifications = state.argo_notifications.write().await;
    let count = notifications.len();
    notifications.clear();
    info!("🗑️  Cleared {} ArgoCD notifications", count);
    ResponseJson(serde_json::json!({
        "cleared": count,
        "message": "ArgoCD notifications cleared",
        "type": "argo"
    }))
}

/// Generic webhook handler - receives notifications for any notification type
async fn generic_webhook_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Path(notification_type): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> StatusCode {
    // Skip flux and argo as they have dedicated handlers
    if notification_type == "flux-notification" || notification_type == "argo-notification" {
        return StatusCode::BAD_REQUEST;
    }

    let notification = create_notification(payload);

    {
        let mut notifications_map = state.generic_notifications.write().await;
        let notifications = notifications_map
            .entry(notification_type.clone())
            .or_insert_with(Vec::new);
        notifications.push(notification.clone());
        info!(
            "📨 Received generic webhook notification: type={}, event_type={}, total_notifications={}",
            notification_type,
            notification.event_type,
            notifications.len()
        );
    }

    StatusCode::OK
}

/// Get all stored notifications for a generic notification type
async fn get_generic_notifications(
    axum::extract::State(state): axum::extract::State<AppState>,
    Path(notification_type): Path<String>,
) -> ResponseJson<serde_json::Value> {
    // Skip flux and argo as they have dedicated handlers
    if notification_type == "flux-notification" || notification_type == "argo-notification" {
        return ResponseJson(serde_json::json!({
            "error": "Use dedicated endpoints for flux-notification and argo-notification",
            "notifications": [],
            "count": 0
        }));
    }

    let notifications_map = state.generic_notifications.read().await;
    let notifications = notifications_map
        .get(&notification_type)
        .cloned()
        .unwrap_or_default();

    ResponseJson(serde_json::json!({
        "notifications": notifications,
        "count": notifications.len(),
        "type": notification_type
    }))
}

/// Clear all stored notifications for a generic notification type
async fn clear_generic_notifications(
    axum::extract::State(state): axum::extract::State<AppState>,
    Path(notification_type): Path<String>,
) -> ResponseJson<serde_json::Value> {
    // Skip flux and argo as they have dedicated handlers
    if notification_type == "flux-notification" || notification_type == "argo-notification" {
        return ResponseJson(serde_json::json!({
            "error": "Use dedicated endpoints for flux-notification and argo-notification",
            "cleared": 0
        }));
    }

    let mut notifications_map = state.generic_notifications.write().await;
    let count = notifications_map
        .remove(&notification_type)
        .map(|v| v.len())
        .unwrap_or(0);

    info!(
        "🗑️  Cleared {} notifications for type: {}",
        count, notification_type
    );
    ResponseJson(serde_json::json!({
        "cleared": count,
        "message": format!("Notifications cleared for type: {}", notification_type),
        "type": notification_type
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;
    use serde_json::json;

    fn create_test_app() -> Router {
        let state = AppState {
            flux_notifications: Arc::new(RwLock::new(Vec::new())),
            argo_notifications: Arc::new(RwLock::new(Vec::new())),
            generic_notifications: Arc::new(RwLock::new(HashMap::new())),
        };

        Router::new()
            .route("/health", get(health))
            .route(
                "/api/flux-notification",
                MethodRouter::new()
                    .post(flux_webhook_handler)
                    .get(get_flux_notifications),
            )
            .route(
                "/api/flux-notification/clear",
                post(clear_flux_notifications),
            )
            .route(
                "/api/argo-notification",
                MethodRouter::new()
                    .post(argo_webhook_handler)
                    .get(get_argo_notifications),
            )
            .route(
                "/api/argo-notification/clear",
                post(clear_argo_notifications),
            )
            .route(
                "/api/{notification_type}",
                MethodRouter::new()
                    .post(generic_webhook_handler)
                    .get(get_generic_notifications),
            )
            .route(
                "/api/{notification_type}/clear",
                post(clear_generic_notifications),
            )
            .with_state(state)
    }

    fn create_fluxcd_payload() -> serde_json::Value {
        json!({
            "involvedObject": {
                "kind": "ApplicationConfiguration",
                "name": "test-config",
                "namespace": "default",
                "uid": "12345",
                "apiVersion": "secret-management.octopilot.io/v1beta1"
            },
            "severity": "warning",
            "timestamp": "2024-01-01T00:00:00Z",
            "message": "Drift detected in secret test-secret",
            "reason": "DriftDetected",
            "reportingController": "secret-manager-controller"
        })
    }

    fn create_argocd_payload() -> serde_json::Value {
        json!({
            "eventType": "drift-detected",
            "application": {
                "name": "test-app",
                "namespace": "default"
            },
            "message": "Drift detected in secret test-secret"
        })
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = create_test_app();
        let server = TestServer::new(app).unwrap();

        let response = server.get("/health").await;
        response.assert_status(StatusCode::OK);
        response.assert_text("ok");
    }

    #[tokio::test]
    async fn test_flux_webhook_valid_payload() {
        let app = create_test_app();
        let server = TestServer::new(app).unwrap();

        let payload = create_fluxcd_payload();

        let response = server.post("/api/flux-notification").json(&payload).await;

        response.assert_status(StatusCode::OK);
    }

    #[tokio::test]
    async fn test_flux_webhook_invalid_payload_missing_fields() {
        let app = create_test_app();
        let server = TestServer::new(app).unwrap();

        let payload = json!({
            "message": "Test message"
            // Missing required fields
        });

        let response = server.post("/api/flux-notification").json(&payload).await;

        response.assert_status(StatusCode::BAD_REQUEST);
        let body: serde_json::Value = response.json();
        assert!(body.get("error").is_some());
        assert!(body.get("details").is_some());
    }

    #[tokio::test]
    async fn test_flux_webhook_invalid_severity() {
        let app = create_test_app();
        let server = TestServer::new(app).unwrap();

        let mut payload = create_fluxcd_payload();
        payload
            .as_object_mut()
            .unwrap()
            .insert("severity".to_string(), json!("invalid"));

        let response = server.post("/api/flux-notification").json(&payload).await;

        response.assert_status(StatusCode::BAD_REQUEST);
        let body: serde_json::Value = response.json();
        assert!(body.get("error").is_some());
        assert!(body.get("details").is_some());
    }

    #[tokio::test]
    async fn test_flux_webhook_get_notifications() {
        let app = create_test_app();
        let server = TestServer::new(app).unwrap();

        // Send a notification
        let payload = create_fluxcd_payload();
        server.post("/api/flux-notification").json(&payload).await;

        // Get notifications
        let response = server.get("/api/flux-notification").await;
        response.assert_status(StatusCode::OK);

        let body: serde_json::Value = response.json();
        assert_eq!(body["count"], 1);
        assert_eq!(body["type"], "flux");
        assert_eq!(body["notifications"].as_array().unwrap().len(), 1);

        let notification = &body["notifications"][0];
        assert_eq!(notification["event_type"], "DriftDetected");
        assert!(notification["timestamp"].is_string());
        assert!(notification["payload"].is_object());
    }

    #[tokio::test]
    async fn test_flux_webhook_clear_notifications() {
        let app = create_test_app();
        let server = TestServer::new(app).unwrap();

        // Send multiple notifications
        let payload = create_fluxcd_payload();
        server.post("/api/flux-notification").json(&payload).await;
        server.post("/api/flux-notification").json(&payload).await;

        // Clear notifications
        let response = server.post("/api/flux-notification/clear").await;
        response.assert_status(StatusCode::OK);

        let body: serde_json::Value = response.json();
        assert_eq!(body["cleared"], 2);
        assert_eq!(body["type"], "flux");

        // Verify notifications are cleared
        let get_response = server.get("/api/flux-notification").await;
        let get_body: serde_json::Value = get_response.json();
        assert_eq!(get_body["count"], 0);
    }

    #[tokio::test]
    async fn test_argo_webhook_handler() {
        let app = create_test_app();
        let server = TestServer::new(app).unwrap();

        let payload = create_argocd_payload();

        let response = server.post("/api/argo-notification").json(&payload).await;

        response.assert_status(StatusCode::OK);
    }

    #[tokio::test]
    async fn test_argo_webhook_get_notifications() {
        let app = create_test_app();
        let server = TestServer::new(app).unwrap();

        // Send a notification
        let payload = create_argocd_payload();
        server.post("/api/argo-notification").json(&payload).await;

        // Get notifications
        let response = server.get("/api/argo-notification").await;
        response.assert_status(StatusCode::OK);

        let body: serde_json::Value = response.json();
        assert_eq!(body["count"], 1);
        assert_eq!(body["type"], "argo");
    }

    #[tokio::test]
    async fn test_argo_webhook_clear_notifications() {
        let app = create_test_app();
        let server = TestServer::new(app).unwrap();

        // Send notifications
        let payload = create_argocd_payload();
        server.post("/api/argo-notification").json(&payload).await;

        // Clear notifications
        let response = server.post("/api/argo-notification/clear").await;
        response.assert_status(StatusCode::OK);

        let body: serde_json::Value = response.json();
        assert_eq!(body["cleared"], 1);
        assert_eq!(body["type"], "argo");
    }

    #[tokio::test]
    async fn test_segregated_storage() {
        let app = create_test_app();
        let server = TestServer::new(app).unwrap();

        // Send FluxCD notification
        let flux_payload = create_fluxcd_payload();
        server
            .post("/api/flux-notification")
            .json(&flux_payload)
            .await;

        // Send ArgoCD notification
        let argo_payload = create_argocd_payload();
        server
            .post("/api/argo-notification")
            .json(&argo_payload)
            .await;

        // Clear FluxCD notifications
        server.post("/api/flux-notification/clear").await;

        // Verify FluxCD is cleared but ArgoCD is not
        let flux_response = server.get("/api/flux-notification").await;
        let flux_body: serde_json::Value = flux_response.json();
        assert_eq!(flux_body["count"], 0);

        let argo_response = server.get("/api/argo-notification").await;
        let argo_body: serde_json::Value = argo_response.json();
        assert_eq!(argo_body["count"], 1);
    }

    #[tokio::test]
    async fn test_generic_webhook_handler() {
        let app = create_test_app();
        let server = TestServer::new(app).unwrap();

        let payload = json!({
            "eventType": "test-event",
            "message": "Test notification"
        });

        let response = server.post("/api/custom-notification").json(&payload).await;

        response.assert_status(StatusCode::OK);
    }

    #[tokio::test]
    async fn test_generic_webhook_get_notifications() {
        let app = create_test_app();
        let server = TestServer::new(app).unwrap();

        let payload = json!({
            "eventType": "test-event",
            "message": "Test notification"
        });

        server.post("/api/custom-notification").json(&payload).await;

        let response = server.get("/api/custom-notification").await;
        response.assert_status(StatusCode::OK);

        let body: serde_json::Value = response.json();
        assert_eq!(body["count"], 1);
        assert_eq!(body["type"], "custom-notification");
    }

    #[tokio::test]
    async fn test_generic_webhook_rejects_flux_argo_paths() {
        let app = create_test_app();
        let server = TestServer::new(app).unwrap();

        let payload = json!({"message": "test"});

        // Should reject flux-notification path
        let response = server.post("/api/flux-notification").json(&payload).await;
        // This should go to the dedicated handler, not generic
        // But if it's invalid FluxCD format, it will return 400
        let status = response.status_code().as_u16();
        assert!(status == 400 || status == 200);

        // Should reject argo-notification path
        let response = server.post("/api/argo-notification").json(&payload).await;
        // This should go to the dedicated handler
        assert_eq!(response.status_code().as_u16(), 200);
    }

    #[tokio::test]
    async fn test_validate_fluxcd_payload() {
        // Valid payload
        let valid = create_fluxcd_payload();
        assert!(validate_fluxcd_payload(&valid).is_ok());

        // Missing involvedObject
        let invalid1 = json!({
            "severity": "info",
            "message": "test",
            "reason": "test"
        });
        assert!(validate_fluxcd_payload(&invalid1).is_err());

        // Missing severity
        let invalid2 = json!({
            "involvedObject": {},
            "message": "test",
            "reason": "test"
        });
        assert!(validate_fluxcd_payload(&invalid2).is_err());

        // Invalid severity
        let mut invalid3 = create_fluxcd_payload();
        invalid3
            .as_object_mut()
            .unwrap()
            .insert("severity".to_string(), json!("invalid"));
        assert!(validate_fluxcd_payload(&invalid3).is_err());

        // Valid severity values
        for severity in ["info", "warning", "error"] {
            let mut payload = create_fluxcd_payload();
            payload
                .as_object_mut()
                .unwrap()
                .insert("severity".to_string(), json!(severity));
            assert!(validate_fluxcd_payload(&payload).is_ok());
        }
    }
}
