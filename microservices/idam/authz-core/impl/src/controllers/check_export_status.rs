use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::check_export_status::{Request, Response};

/// Handler for Check Export Status — exports audit events from the org..
#[handler(CheckExportStatusController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditActor, AuditEvent, AuditEventType, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::Compliance,
        "export_status_checked",
        req.data.x_tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.metadata = serde_json::json!({ "export_id": req.data.export_id }).into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    // TODO: Check status in Redis/DB for the export job

    Response {
        export_id: req.data.export_id,
        status: "pending".to_string(),
        download_url: None,
        estimated_completion: None,
    }
}
