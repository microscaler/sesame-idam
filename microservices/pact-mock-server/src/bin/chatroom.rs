use axum::{
    extract::{
        ws::{Message, Utf8Bytes, WebSocket},
        Query, WebSocketUpgrade,
    },
    http::{Method, StatusCode},
    middleware,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use futures::stream::StreamExt;
use futures::SinkExt;
use pact_mock_server::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// ============================================================
// Shared State
// ============================================================

#[derive(Clone, Debug)]
pub struct ChatState {
    pub rooms: Arc<RwLock<HashMap<String, ChatRoom>>>,
    pub messages: Arc<RwLock<HashMap<String, Vec<ChatMessage>>>>,
    pub users: Arc<RwLock<HashMap<String, UserInfo>>>,
}

impl ChatState {
    fn new() -> Self {
        Self {
            rooms: Arc::new(RwLock::new(HashMap::new())),
            messages: Arc::new(RwLock::new(HashMap::new())),
            users: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ChatRoom {
    id: String,
    name: String,
    room_type: String, // "project" | "story" | "agent"
    description: String,
    agent_ids: Vec<String>,
    created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ChatMessage {
    id: String,
    room_id: String,
    sender_id: String,
    sender_type: String, // "human" | "agent"
    content: String,
    ts: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UserInfo {
    user_id: String,
    display_name: String,
    persona: String, // agent persona string or "human"
}

// ============================================================
// Request/Response DTOs
// ============================================================

#[derive(Deserialize)]
struct UploadUrlParams {
    namespace: String,
    target_id: String,
}

#[derive(Deserialize)]
struct DownloadUrlParams {
    namespace: String,
    target_id: String,
    document_id: String,
}

#[derive(Deserialize)]
struct ComplianceDocumentPayload {
    id: String,
}

// Chat request DTOs
#[derive(Deserialize)]
struct CreateRoomRequest {
    name: String,
    room_type: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    agent_ids: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct SendMessageRequest {
    content: String,
    #[serde(default)]
    sender_id: Option<String>,
    #[serde(default)]
    sender_type: Option<String>,
}

#[derive(Deserialize)]
struct AuthHeaderParams {
    user_id: Option<String>,
    token: Option<String>,
}

// ============================================================
// Helpers — Response wrapper for chat handlers
// ============================================================

/// Response enum for chat handlers that can return either
/// a JSON body or a JSON body with a status code.
#[derive(Debug)]
enum ChatResponse {
    Ok(Json<serde_json::Value>),
    StatusCode(StatusCode, Json<serde_json::Value>),
}

impl IntoResponse for ChatResponse {
    fn into_response(self) -> axum::response::Response {
        match self {
            ChatResponse::Ok(body) => body.into_response(),
            ChatResponse::StatusCode(code, body) => (code, body).into_response(),
        }
    }
}

/// Authenticate user from headers or query params.
/// Returns (user_id, persona). Returns defaults for guest.
fn resolve_auth(params: &AuthHeaderParams) -> (String, String) {
    (
        params
            .user_id
            .clone()
            .unwrap_or_else(|| "user-guest".to_string()),
        params
            .token
            .as_ref()
            .map(|t| t.clone())
            .unwrap_or_else(|| "human".to_string()),
    )
}

/// Helper: create a room response
fn room_response(room: &ChatRoom) -> ChatResponse {
    ChatResponse::Ok(Json(json!({
        "id": room.id,
        "name": room.name,
        "room_type": room.room_type,
        "description": room.description,
        "agent_ids": room.agent_ids,
        "created_at": room.created_at
    })))
}

/// Helper: send a message and return response
fn message_response(msg: &ChatMessage) -> ChatResponse {
    ChatResponse::StatusCode(
        StatusCode::CREATED,
        Json(json!({
            "id": msg.id,
            "room_id": msg.room_id,
            "sender_id": msg.sender_id,
            "sender_type": msg.sender_type,
            "content": msg.content,
            "ts": msg.ts
        })),
    )
}

// ============================================================
// Handlers — Chat (REST)
// ============================================================

/// Create a new chat room (project-level or story-level)
async fn create_room(
    axum::extract::State(state): axum::extract::State<ChatState>,
    Json(req): Json<CreateRoomRequest>,
) -> impl IntoResponse {
    let room_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let room = ChatRoom {
        id: room_id.clone(),
        name: req.name.clone(),
        room_type: req.room_type.clone(),
        description: req.description.unwrap_or_default(),
        agent_ids: req.agent_ids.unwrap_or_default(),
        created_at: now.clone(),
    };
    state.rooms.write().await.insert(room_id.clone(), room);
    state.messages.write().await.insert(room_id.clone(), vec![]);

    info!("Room created: {} (type={})", room_id, req.room_type);

    (
        StatusCode::CREATED,
        Json(json!({
            "id": room_id,
            "name": req.name,
            "room_type": req.room_type,
            "description": "",
            "agent_ids": [],
            "created_at": now
        })),
    )
}

/// List rooms for a project (query param: project_id)
async fn list_rooms(
    axum::extract::State(state): axum::extract::State<ChatState>,
    Query(params): Query<AuthHeaderParams>,
) -> impl IntoResponse {
    let (user_id, _) = resolve_auth(&params);
    let rooms = state.rooms.read().await;
    let room_list: Vec<_> = rooms
        .values()
        .map(|r| {
            json!({
                "id": r.id,
                "name": r.name,
                "room_type": r.room_type,
                "description": r.description,
                "agent_ids": r.agent_ids,
                "created_at": r.created_at
            })
        })
        .collect();

    info!("List rooms: {} rooms, user={}", room_list.len(), user_id);

    Json(json!({
        "rooms": room_list,
        "total": room_list.len()
    }))
}

/// Get a single room by ID
async fn get_room(
    axum::extract::State(state): axum::extract::State<ChatState>,
    axum::extract::Path(room_id): axum::extract::Path<String>,
    Query(params): Query<AuthHeaderParams>,
) -> ChatResponse {
    let (user_id, _) = resolve_auth(&params);
    let rooms = state.rooms.read().await;
    match rooms.get(&room_id) {
        Some(room) => {
            info!("Get room: {} user={}", room_id, user_id);
            room_response(room)
        }
        None => ChatResponse::StatusCode(
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Room not found"})),
        ),
    }
}

/// Delete a room
async fn delete_room(
    axum::extract::State(state): axum::extract::State<ChatState>,
    axum::extract::Path(room_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let mut rooms = state.rooms.write().await;
    if rooms.remove(&room_id).is_some() {
        state.messages.write().await.remove(&room_id);
        info!("Room deleted: {}", room_id);
        (StatusCode::NO_CONTENT, Json(json!({"deleted": room_id})))
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Room not found"})),
        )
    }
}

/// Send a message to a room
async fn send_message(
    axum::extract::State(state): axum::extract::State<ChatState>,
    axum::extract::Path(room_id): axum::extract::Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> ChatResponse {
    // Verify room exists
    let room_exists = state.rooms.read().await.contains_key(&room_id);
    if !room_exists {
        return ChatResponse::StatusCode(
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Room not found"})),
        );
    }

    let msg_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let sender_id = req.sender_id.unwrap_or_else(|| "system".to_string());
    let sender_type = req.sender_type.unwrap_or_else(|| "human".to_string());

    let msg = ChatMessage {
        id: msg_id.clone(),
        room_id: room_id.clone(),
        sender_id: sender_id.clone(),
        sender_type: sender_type.clone(),
        content: req.content.clone(),
        ts: now.clone(),
    };

    state
        .messages
        .write()
        .await
        .entry(room_id.clone())
        .or_default()
        .push(msg.clone());

    info!(
        "Message sent: {} room={} sender={} type={}",
        msg_id, room_id, sender_id, sender_type
    );

    message_response(&msg)
}

/// Get message history for a room (with optional limit/since)
async fn get_room_messages(
    axum::extract::State(state): axum::extract::State<ChatState>,
    axum::extract::Path(room_id): axum::extract::Path<String>,
    Query(params): Query<AuthHeaderParams>,
) -> impl IntoResponse {
    let (user_id, _) = resolve_auth(&params);
    let messages = state.messages.read().await;
    let msgs = messages.get(&room_id).cloned().unwrap_or_default();

    let msg_list: Vec<_> = msgs
        .iter()
        .map(|m| {
            json!({
                "id": m.id,
                "sender_id": m.sender_id,
                "sender_type": m.sender_type,
                "content": m.content,
                "ts": m.ts
            })
        })
        .collect();

    info!(
        "Messages: {} for room={} user={}",
        msg_list.len(),
        room_id,
        user_id
    );

    Json(json!({
        "messages": msg_list,
        "total": msg_list.len()
    }))
}

/// Join an agent to a room
async fn join_room(
    axum::extract::State(state): axum::extract::State<ChatState>,
    axum::extract::Path((room_id, agent_id)): axum::extract::Path<(String, String)>,
    Query(params): Query<AuthHeaderParams>,
) -> impl IntoResponse {
    let (user_id, _) = resolve_auth(&params);
    let mut rooms = state.rooms.write().await;
    if let Some(room) = rooms.get_mut(&room_id) {
        if !room.agent_ids.contains(&agent_id) {
            room.agent_ids.push(agent_id.clone());
        }
        // Register the agent as a user
        state.users.write().await.insert(
            agent_id.clone(),
            UserInfo {
                user_id: agent_id.clone(),
                display_name: format!("Agent {}", agent_id.chars().take(8).collect::<String>()),
                persona: "agent".to_string(),
            },
        );
        info!("Agent {} joined room {} by {}", agent_id, room_id, user_id);
        return (
            StatusCode::OK,
            Json(json!({
                "room_id": room_id,
                "agent_id": agent_id,
                "agent_ids": room.agent_ids,
                "joined": true
            })),
        );
    }
    (
        StatusCode::NOT_FOUND,
        Json(json!({"error": "Room not found"})),
    )
}

/// List agents in a room
async fn list_room_agents(
    axum::extract::State(state): axum::extract::State<ChatState>,
    axum::extract::Path(room_id): axum::extract::Path<String>,
    Query(params): Query<AuthHeaderParams>,
) -> ChatResponse {
    let _ = params;
    let rooms = state.rooms.read().await;
    match rooms.get(&room_id) {
        Some(room) => {
            let users = state.users.read().await;
            let agent_info: Vec<_> = room
                .agent_ids
                .iter()
                .filter_map(|aid| {
                    users.get(aid).map(|u| {
                        json!({
                            "user_id": u.user_id,
                            "display_name": u.display_name,
                            "persona": u.persona
                        })
                    })
                })
                .collect();
            ChatResponse::Ok(Json(json!({"agents": agent_info})))
        }
        None => ChatResponse::StatusCode(
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Room not found"})),
        ),
    }
}

// ============================================================
// Handlers — Storage (existing)
// ============================================================

async fn generate_upload_url(Json(payload): Json<UploadUrlParams>) -> impl IntoResponse {
    let document_id = uuid::Uuid::new_v4().to_string();
    let url = format!(
        "http://127.0.0.1:9000/mock-bucket/{}/{}/doc_{}?X-Amz-Signature=***",
        payload.namespace, payload.target_id, document_id
    );

    (
        StatusCode::OK,
        Json(json!({
            "document_id": document_id,
            "upload_url": url,
            "expiry_duration_secs": 900
        })),
    )
}

async fn generate_download_url(Query(params): Query<DownloadUrlParams>) -> impl IntoResponse {
    let url = format!(
        "http://127.0.0.1:9000/mock-bucket/{}/{}/doc_{}?X-Amz-Signature=***",
        params.namespace, params.target_id, params.document_id
    );

    (
        StatusCode::OK,
        Json(json!({
            "download_url": url,
            "expiry_duration_secs": 900
        })),
    )
}

async fn register_compliance_document(
    Json(payload): Json<ComplianceDocumentPayload>,
) -> impl IntoResponse {
    (
        StatusCode::CREATED,
        Json(json!({
            "id": payload.id,
            "status": "PENDING_VERIFICATION",
            "updated_at": "2026-04-12T12:00:00Z"
        })),
    )
}

// ============================================================
// WebSocket handler — real-time chat streaming
// ============================================================

async fn chat_ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<AuthHeaderParams>,
    axum::extract::Path(room_id): axum::extract::Path<String>,
    axum::extract::State(state): axum::extract::State<ChatState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, params, room_id, state))
}

async fn handle_ws(socket: WebSocket, params: AuthHeaderParams, room_id: String, state: ChatState) {
    let (mut tx, mut rx) = socket.split();
    let (user_id, _) = resolve_auth(&params);
    info!("WebSocket connected: user={} room={}", user_id, room_id);

    // Check room exists
    let rooms = state.rooms.read().await;
    let exists = rooms.contains_key(&room_id);
    drop(rooms);

    if !exists {
        if let Err(e) = tx
            .send(Message::Text(Utf8Bytes::from(
                json!({"error": "Room not found"}).to_string(),
            )))
            .await
        {
            info!("WebSocket send error (room not found): {}", e);
        }
        return;
    }

    // Send join confirmation
    if let Err(e) = tx
        .send(Message::Text(Utf8Bytes::from(
            json!({
                "event": "connected",
                "room_id": room_id,
                "user_id": user_id
            })
            .to_string(),
        )))
        .await
    {
        info!("WebSocket send error (join confirmation): {}", e);
        return;
    }

    // Send existing message history (last 50 messages)
    let history_lock = state.messages.read().await;
    let history: Vec<_> = history_lock
        .get(&room_id)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .rev()
        .take(50)
        .map(|m| {
            json!({
                "event": "message",
                "id": m.id,
                "sender_id": m.sender_id,
                "sender_type": m.sender_type,
                "content": m.content,
                "ts": m.ts
            })
        })
        .collect();
    drop(history_lock);

    for msg in history {
        if let Err(e) = tx
            .send(Message::Text(Utf8Bytes::from(msg.to_string())))
            .await
        {
            info!("WebSocket send error (history): {}", e);
            return;
        }
    }

    // Stream loop: forward incoming WS messages, broadcast to room
    let broadcast_room = room_id.clone();
    let broadcast_user = user_id.clone();

    while let Some(msg) = rx.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let payload = serde_json::from_str::<serde_json::Value>(&text);
                match payload {
                    Ok(p) => {
                        let content = p.get("content").and_then(|c| c.as_str()).unwrap_or("");
                        let sender_type = p
                            .get("sender_type")
                            .and_then(|s| s.as_str())
                            .unwrap_or("human");
                        let sender_id = p
                            .get("sender_id")
                            .and_then(|s| s.as_str())
                            .unwrap_or(&broadcast_user);

                        let msg_id = uuid::Uuid::new_v4().to_string();
                        let now = chrono::Utc::now().to_rfc3339();

                        let chat_msg = ChatMessage {
                            id: msg_id.clone(),
                            room_id: broadcast_room.clone(),
                            sender_id: sender_id.to_string(),
                            sender_type: sender_type.to_string(),
                            content: content.to_string(),
                            ts: now.clone(),
                        };

                        state
                            .messages
                            .write()
                            .await
                            .entry(broadcast_room.clone())
                            .or_default()
                            .push(chat_msg);

                        // Broadcast back as new message
                        let broadcast = json!({
                            "event": "message",
                            "id": msg_id,
                            "sender_id": sender_id,
                            "sender_type": sender_type,
                            "content": content,
                            "ts": now
                        });
                        if let Err(e) = tx
                            .send(Message::Text(Utf8Bytes::from(broadcast.to_string())))
                            .await
                        {
                            info!("WebSocket broadcast error: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        info!("WebSocket invalid JSON: {}", e);
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("WebSocket closed: user={} room={}", user_id, room_id);
                break;
            }
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(vec![
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers(Any);

    let chat_state = ChatState::new();

    let app = Router::new()
        .route("/health", get(health_check))
        .route(
            "/api/v1/storage/documents/upload-url",
            post(generate_upload_url),
        )
        .route(
            "/api/v1/storage/documents/download-url",
            get(generate_download_url),
        )
        .route(
            "/api/v1/company/organizations/me/compliance/documents",
            post(register_compliance_document),
        )
        // ================================================================
        // Chat REST endpoints
        // ================================================================
        .route("/api/v1/chat/rooms", get(list_rooms).post(create_room))
        .route(
            "/api/v1/chat/rooms/:room_id",
            get(get_room).delete(delete_room),
        )
        .route(
            "/api/v1/chat/rooms/:room_id/messages",
            get(get_room_messages).post(send_message),
        )
        .route(
            "/api/v1/chat/rooms/:room_id/agents/:agent_id/join",
            post(join_room),
        )
        .route("/api/v1/chat/rooms/:room_id/agents", get(list_room_agents))
        // Chat WebSocket for real-time streaming
        .route("/api/v1/chat/ws/:room_id", get(chat_ws_handler))
        .layer(middleware::from_fn(logging_middleware))
        .layer(middleware::from_fn(service_unavailable_middleware))
        .layer(middleware::from_fn(rate_limit_middleware))
        .layer(middleware::from_fn(auth_failure_middleware))
        .layer(cors)
        .with_state(chat_state);

    let port = 8017;
    let addr = format!("0.0.0.0:{}", port);

    info!("🚀 Starting Hauliage Pact Mock Server on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
