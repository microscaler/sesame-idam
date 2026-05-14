// User-owned controller for handler 'check_export_status'.

use crate::handlers::check_export_status::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(CheckExportStatusController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "download_url": "https://storage.example.com/exports/audit-6ba7b810.json",
    //   "estimated_completion": null,
    //   "export_id": "6ba7b810-9dad-11d1-80b4-00c04fd430cb",
    //   "status": "complete"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "download_url": "https://storage.example.com/exports/audit-6ba7b810.json",
  "estimated_completion": null,
  "export_id": "6ba7b810-9dad-11d1-80b4-00c04fd430cb",
  "status": "complete"
}"###,
    ) {
        Ok(parsed) => return parsed,
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    Response {
        download_url: Some("https://storage.example.com/exports/audit-6ba7b810.json".to_string()),
        estimated_completion: Some("example".to_string()),
        export_id: "6ba7b810-9dad-11d1-80b4-00c04fd430cb".to_string(),
        status: "complete".to_string(),
    }
}
