//! What assign, revoke and list share: the caller (a tenant administrator, from the token only), the audit event and
//! the version bump that tells caches a tenant's roles changed.
use brrtrouter::typed::HttpJson;
use serde_json::{json, Value};
use sesame_common::tenant_admin::{tenant_admin, TenantAdmin};

pub fn bad_request(message: &str) -> HttpJson<Value> {
    HttpJson::new(
        400,
        json!({ "error": "invalid_request", "error_description": message }),
    )
}

pub fn not_found(message: &str) -> HttpJson<Value> {
    HttpJson::new(
        404,
        json!({ "error": "not_found", "error_description": message }),
    )
}

pub fn internal(op: &str, e: &dyn std::fmt::Display) -> HttpJson<Value> {
    tracing::error!(error = %e, "{op}: database error");
    HttpJson::new(
        500,
        json!({ "error": "internal_error", "error_description": "An unexpected error occurred" }),
    )
}

/// The caller, who must administer the tenant named by the header and the body (when they name one).
pub fn admin(
    claims: &Option<Value>,
    named: &[Option<&str>],
) -> Result<TenantAdmin, HttpJson<Value>> {
    let a = tenant_admin(claims)?;
    for n in named {
        a.require_same_tenant(*n)?;
    }
    Ok(a)
}

pub fn parse_uuid(raw: &str, what: &str) -> Result<uuid::Uuid, HttpJson<Value>> {
    raw.trim()
        .parse::<uuid::Uuid>()
        .map_err(|_| bad_request(&format!("{what} is not a UUID")))
}

/// 403 with the role policy's reason (sesame_common::role_policy).
pub fn refused(r: &sesame_common::role_policy::Refusal) -> HttpJson<Value> {
    sesame_common::tenant_admin::forbidden(r.message())
}

/// Make every token the user holds stale (Sesame's token status check refuses a token older than its subject's
/// version), so a role change applies to the next request rather than the next refresh.
///
/// # Errors
/// The version store could not be reached.
pub fn invalidate_tokens(user_id: uuid::Uuid) -> Result<u64, String> {
    sesame_common::VersionStore::from_env()
        .and_then(|store| store.increment_subject(&user_id.to_string()))
        .map_err(|e| e.to_string())
}

/// Audit (Delegation) and the tenant's version bump, for an assignment that changed.
pub fn record_change(
    admin: &TenantAdmin,
    user_id: uuid::Uuid,
    role: &str,
    decision: &str,
    reason: sesame_common::token_versioning::BumpReason,
) {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEventType, AuditLogEntry};
    let mut metadata = serde_json::Map::new();
    metadata.insert("role".into(), json!(role));
    metadata.insert("by".into(), json!(admin.user_id.to_string()));
    if let Ok(entry) = AuditLogEntry::new(AuditEventType::Delegation, "authz-core")
        .decision_source(decision)
        .tenant_id(&admin.tenant)
        .user_id(user_id.to_string())
        .metadata(Value::Object(metadata))
        .build()
    {
        EMITTER.emit(entry);
    }
    if let Some(publisher) = &*crate::audit::PUBLISHER {
        publisher.publish_tenant(&admin.tenant, 0, reason);
    }
}
