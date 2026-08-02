use brrtrouter::typed::HttpJson;
use sesame_common::audit::{AuditEventType, AuditLogEntry};

use crate::audit::EMITTER;
use crate::services::oidc_client_admin::{ClientAdminError, ClientView};
use crate::services::tenant_admin::TenantAdmin;

#[must_use]
pub fn client_json(client: &ClientView) -> serde_json::Value {
    serde_json::json!({
        "client_id": client.client_id,
        "tenant_id": client.tenant_id,
        "application_id": client.application_id,
        "client_type": client.client_type,
        "token_endpoint_auth_method": client.token_endpoint_auth_method,
        "pkce_s256_required": client.pkce_s256_required,
        "status": client.status,
        "redirect_uris": client.redirect_uris,
        "post_logout_redirect_uris": client.post_logout_redirect_uris,
        "grants": client.grants,
        "response_types": client.response_types,
        "scopes": client.scopes,
        "audiences": client.audiences,
        "created_at": client.created_at.to_rfc3339(),
        "updated_at": client.updated_at.to_rfc3339(),
    })
}

#[must_use]
pub fn admin_error(error: ClientAdminError) -> HttpJson<serde_json::Value> {
    let (code, message) = match &error {
        ClientAdminError::InvalidPolicy(message) => ("invalid_client_metadata", message.clone()),
        ClientAdminError::NotFound => ("not_found", "OIDC client not found".to_string()),
        ClientAdminError::PublicClientHasNoSecret => (
            "invalid_client_metadata",
            "Public clients cannot have credentials".to_string(),
        ),
        ClientAdminError::InvalidOverlap => (
            "invalid_request",
            "Secret overlap must be between 0 and 86400 seconds".to_string(),
        ),
        ClientAdminError::Db(message) => {
            tracing::error!(error = %message, "OIDC client lifecycle database failure");
            ("internal_error", "An unexpected error occurred".to_string())
        }
    };
    HttpJson::new(
        error.status(),
        serde_json::json!({ "error": code, "error_description": message }),
    )
}

pub fn emit_lifecycle_audit(action: &str, admin: &TenantAdmin, client_id: &str, outcome: &str) {
    let entry = AuditLogEntry::new(AuditEventType::Delegation, "identity-login-service")
        .tenant_id(admin.tenant.clone())
        .user_id(admin.user_id.to_string())
        .metadata(serde_json::json!({
            "action": action,
            "client_id": client_id,
            "outcome": outcome,
            "authority_class": "tenant",
        }))
        .build();
    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_errors_are_redacted_from_public_responses() {
        let response = admin_error(ClientAdminError::Db(
            "secret_hash=$argon2id$private database=internal".to_string(),
        ));
        let serialized = serde_json::to_string(&response.body).expect("serialize response");
        assert!(!serialized.contains("argon2id"));
        assert!(!serialized.contains("database=internal"));
        assert!(serialized.contains("internal_error"));
    }
}
