// User-owned controller for handler 'export_audit_events'.

use crate::handlers::export_audit_events::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(ExportAuditEventsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "download_url": null,
    //   "estimated_completion": "2026-05-11T14:35:00Z",
    //   "export_id": "6ba7b810-9dad-11d1-80b4-00c04fd430cb",
    //   "status": "pending"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "download_url": null,
  "estimated_completion": "2026-05-11T14:35:00Z",
  "export_id": "6ba7b810-9dad-11d1-80b4-00c04fd430cb",
  "status": "pending"
}"###,
    ) {
        Ok(parsed) => return parsed,
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    Response {
        download_url: Some("example".to_string()),
        estimated_completion: Some("2026-05-11T14:35:00Z".to_string()),
        export_id: "6ba7b810-9dad-11d1-80b4-00c04fd430cb".to_string(),
        status: "pending".to_string(),
    }
}
