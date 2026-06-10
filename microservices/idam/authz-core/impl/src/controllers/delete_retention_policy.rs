use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::delete_retention_policy::{Request, Response};

/// Handler for Delete Retention Policy — deletes an audit log retention policy..
#[handler(DeleteRetentionPolicyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEventType, AuditLogEntry};

    let mut metadata = serde_json::Map::new();
    metadata.insert("policy_id".to_string(), serde_json::json!(&req.data.id));

    let entry = AuditLogEntry::new(AuditEventType::Delegation, "retention_policy_deleted")
        .tenant_id(&req.data.x_tenant_id)
        .metadata(serde_json::Value::Object(metadata))
        .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    // TODO: DELETE FROM retention_policies WHERE id = $1 AND tenant_id = $2

    Response {
        error: String::new(),
        error_description: None,
    }
}
