use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::export_audit_events::{Request, Response};

/// Handler for Export Audit Events
#[handler(ExportAuditEventsController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use uuid::Uuid;

    let export_id = Uuid::new_v4();

    Response {
        export_id: export_id.to_string(),
        status: "pending".to_string(),
        download_url: None,
        estimated_completion: None,
    }
}
