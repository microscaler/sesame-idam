//! `POST /api-keys` — create an M2M API key (DB-backed).
//!
//! The plaintext key is returned exactly once in this response; only its
//! SHA-256 hash is stored.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_api_keys_gen::handlers::create_api_key::Request;
use uuid::Uuid;

use crate::audit::EMITTER;
use crate::services::api_key_service::{ApiKeyService, NewApiKey};
use sesame_common::audit::{AuditEventType, AuditLogEntry};

#[handler(CreateApiKeyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let tenant_id = req.data.x_tenant_id.clone();

    let parse_uuid = |value: Option<&serde_json::Value>| -> Result<Option<Uuid>, String> {
        match value.and_then(|v| v.as_str()) {
            Some(s) => s
                .parse::<Uuid>()
                .map(Some)
                .map_err(|_| format!("'{s}' is not a valid uuid")),
            None => Ok(None),
        }
    };

    let user_id = match parse_uuid(req.data.user_id.as_ref()) {
        Ok(id) => id,
        Err(msg) => return bad_request(&format!("user_id: {msg}")),
    };
    let org_id = match parse_uuid(req.data.org_id.as_ref()) {
        Ok(id) => id,
        Err(msg) => return bad_request(&format!("org_id: {msg}")),
    };

    if user_id.is_none() && org_id.is_none() {
        return bad_request("one of user_id or org_id is required (key scope)");
    }

    let expires_in_days = req
        .data
        .expires_in_days
        .as_ref()
        .and_then(serde_json::Value::as_i64);
    if let Some(days) = expires_in_days {
        if days <= 0 {
            return bad_request("expires_in_days must be positive");
        }
    }

    let params = NewApiKey {
        tenant_id: tenant_id.clone(),
        name: req.data.name.clone(),
        user_id,
        org_id,
        permissions: req.data.permissions.clone(),
        expires_in_days,
    };

    let exec = sesame_idam_database::db();
    let created = match ApiKeyService::create(params, exec) {
        Ok(created) => created,
        Err(e) => {
            tracing::error!(error = %e, "create_api_key: insert failed");
            return HttpJson::new(
                500,
                serde_json::json!({
                    "error": "internal_error",
                    "error_description": "An unexpected error occurred"
                }),
            );
        }
    };

    // Audit — never log the plaintext key.
    let entry = AuditLogEntry::new(AuditEventType::JwtIssued, "api-keys")
        .tenant_id(tenant_id)
        .decision_source("create_api_key")
        .result("allowed")
        .reason(format!("api_key_id={}", created.model.id))
        .build();
    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    let model = &created.model;
    HttpJson::new(
        201,
        serde_json::json!({
            "api_key": created.plaintext,
            "api_key_id": model.id,
            "name": model.name,
            "user_id": model.user_id,
            "org_id": model.org_id,
            "permissions": crate::services::api_key_service::decode_permissions(model),
            "created_at": model.created_at.timestamp(),
            "expires_at": model.expires_at.map(|t| t.timestamp()),
        }),
    )
}

fn bad_request(desc: &str) -> HttpJson<serde_json::Value> {
    HttpJson::new(
        400,
        serde_json::json!({
            "error": "invalid_request",
            "error_description": desc,
        }),
    )
}
