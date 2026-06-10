use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::export_audit_events::{Request, Response};

/// Handler for Export Audit Events — exports audit events from the org..
#[handler(ExportAuditEventsController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEventType, AuditLogEntry};
    use uuid::Uuid;

    let export_id = Uuid::new_v4();

    let mut metadata = serde_json::Map::new();
    metadata.insert(
        "export_id".to_string(),
        serde_json::json!(export_id.to_string()),
    );
    metadata.insert("format".to_string(), serde_json::json!(&req.data.format));
    metadata.insert(
        "include_metadata".to_string(),
        serde_json::json!(req.data.include_metadata),
    );

    let entry = AuditLogEntry::new(AuditEventType::Delegation, "audit_export_requested")
        .tenant_id(&req.data.x_tenant_id)
        .metadata(serde_json::Value::Object(metadata))
        .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    // TODO: Async export task:
    // 1. Spawn background job to write CSV/JSON to S3
    // 2. Return export_id + download_url for polling

    Response {
        export_id: export_id.to_string(),
        status: "pending".to_string(),
        download_url: None,
        estimated_completion: None,
    }
}
