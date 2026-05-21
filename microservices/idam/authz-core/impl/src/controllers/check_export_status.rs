use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::check_export_status::{Request, Response};

/// Handler for Check Export Status — checks status of an audit event export..
#[handler(CheckExportStatusController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEventType, AuditLogEntry};

    let mut metadata = serde_json::Map::new();
    metadata.insert("export_id".to_string(), serde_json::json!(&req.data.export_id));

    let entry = AuditLogEntry::new(AuditEventType::Delegation, "export_status_checked")
        .tenant_id(&req.data.x_tenant_id)
        .metadata(serde_json::Value::Object(metadata))
        .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    // TODO: Check status in Redis/DB for the export job

    Response {
        export_id: req.data.export_id,
        status: "pending".to_string(),
        download_url: None,
        estimated_completion: None,
    }
}
