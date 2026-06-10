use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::create_retention_policy::{Request, Response};

/// Handler for Create Retention Policy — creates an audit log retention policy..
#[handler(CreateRetentionPolicyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEventType, AuditLogEntry};
    use uuid::Uuid;

    let policy_id = Uuid::new_v4();

    let mut metadata = serde_json::Map::new();
    metadata.insert("policy_id".to_string(), serde_json::json!(policy_id.to_string()));
    metadata.insert("event_type".to_string(), serde_json::json!(&req.data.event_type));
    metadata.insert("retention_days".to_string(), serde_json::json!(req.data.retention_days));

    let entry = AuditLogEntry::new(AuditEventType::Delegation, "retention_policy_created")
        .tenant_id(&req.data.x_tenant_id)
        .metadata(serde_json::Value::Object(metadata))
        .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    // TODO: INSERT INTO retention_policies VALUES ($1, $2, ...)

    Response {
        id: Some(policy_id.to_string()),
        event_type: req.data.event_type,
        retention_days: req.data.retention_days,
        archive_after_days: req.data.archive_after_days,
        delete_after_days: req.data.delete_after_days,
        created_at: Some(chrono::Utc::now().to_rfc3339()),
        tenant_id: req.data.x_tenant_id,
    }
}
