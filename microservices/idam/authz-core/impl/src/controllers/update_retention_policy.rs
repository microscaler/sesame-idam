use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::update_retention_policy::{Request, Response};

/// Handler for Update Retention Policy — updates an existing audit log retention policy..
#[handler(UpdateRetentionPolicyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEventType, AuditLogEntry};

    let mut metadata = serde_json::Map::new();
    metadata.insert("policy_id".to_string(), serde_json::json!(&req.data.id));
    if let Some(retention_days) = req.data.retention_days {
        metadata.insert(
            "retention_days".to_string(),
            serde_json::json!(retention_days),
        );
    }

    let entry = AuditLogEntry::new(AuditEventType::Delegation, "retention_policy_updated")
        .tenant_id(&req.data.x_tenant_id)
        .metadata(serde_json::Value::Object(metadata))
        .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    // TODO: UPDATE retention_policies SET retention_days = $1, ...
    // WHERE id = $2 AND tenant_id = $3

    let retention_days = req.data.retention_days.unwrap_or(90);

    Response {
        id: Some(req.data.id),
        event_type: "".to_string(),
        retention_days,
        archive_after_days: req.data.archive_after_days,
        delete_after_days: req.data.delete_after_days,
        created_at: None,
        tenant_id: req.data.x_tenant_id,
    }
}
