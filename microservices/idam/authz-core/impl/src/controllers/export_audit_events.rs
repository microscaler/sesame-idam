use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::export_audit_events::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

/// Handler for Export Audit Events — exports audit events from the org..
#[handler(ExportAuditEventsController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let export_id = Uuid::new_v4();
    
    let mut event = AuditEvent::new(
        AuditEventType::Compliance,
        "audit_export_requested",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.metadata = serde_json::json!({
        "export_id": export_id.to_string(),
        "format": req.inner.format,
        "include_metadata": req.inner.include_metadata,
    }).into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

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
