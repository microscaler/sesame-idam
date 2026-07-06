//! `POST /api-keys/validate` — validate an M2M API key (DB-backed).
//!
//! Tenant-scoped SHA-256 hash lookup with active/expiry/key-type checks.
//! Invalid or type-mismatched keys return 401 per spec; expired keys return
//! 200 with `valid: false, is_expired: true` so callers can distinguish
//! rotation from revocation.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_api_keys_gen::handlers::validate_api_key::Request;

use crate::audit::EMITTER;
use crate::services::api_key_service::{decode_permissions, ApiKeyService, ValidationOutcome};
use sesame_common::audit::{AuditEventType, AuditLogEntry};

#[handler(ValidateApiKeyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let tenant_id = req.data.x_tenant_id.clone();
    let key_type = req.data.key_type.as_deref().unwrap_or("any");

    let exec = sesame_idam_database::db();
    let outcome = match ApiKeyService::validate(&tenant_id, &req.data.api_key, key_type, exec) {
        Ok(outcome) => outcome,
        Err(e) => {
            tracing::error!(error = %e, "validate_api_key: lookup failed");
            return HttpJson::new(
                500,
                serde_json::json!({
                    "error": "internal_error",
                    "error_description": "An unexpected error occurred"
                }),
            );
        }
    };

    let (result, response) = match outcome {
        ValidationOutcome::Valid(key) => {
            let scope_type = if key.user_id.is_some() { "personal" } else { "org" };
            (
                "allowed",
                HttpJson::new(
                    200,
                    serde_json::json!({
                        "valid": true,
                        "is_expired": false,
                        "api_key_id": key.id,
                        "user_id": key.user_id,
                        "org_id": key.org_id,
                        "scope_type": scope_type,
                        "permissions": decode_permissions(&key),
                        "expires_at": key.expires_at.map(|t| t.timestamp()),
                    }),
                ),
            )
        }
        ValidationOutcome::Expired(key) => (
            "denied",
            HttpJson::new(
                200,
                serde_json::json!({
                    "valid": false,
                    "is_expired": true,
                    "api_key_id": key.id,
                    "expires_at": key.expires_at.map(|t| t.timestamp()),
                }),
            ),
        ),
        ValidationOutcome::Invalid => (
            "denied",
            HttpJson::new(
                401,
                serde_json::json!({
                    "error": "invalid_request",
                    "error_description": "Unauthorized — invalid, expired, or missing credentials",
                    "valid": false,
                }),
            ),
        ),
    };

    let entry = AuditLogEntry::new(
        if result == "allowed" {
            AuditEventType::JwtValidated
        } else {
            AuditEventType::ValidationFailed
        },
        "api-keys",
    )
    .tenant_id(tenant_id)
    .decision_source("validate_api_key")
    .result(result)
    .build();
    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    response
}
