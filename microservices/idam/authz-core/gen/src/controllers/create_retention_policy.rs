// User-owned controller for handler 'create_retention_policy'.

use crate::handlers::create_retention_policy::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(CreateRetentionPolicyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    // Example response:
    // {
    //   "archive_after_days": 60,
    //   "created_at": "2026-05-11T14:30:00Z",
    //   "delete_after_days": 365,
    //   "event_type": "user_management",
    //   "id": "6ba7b810-9dad-11d1-80b4-00c04fd430ce",
    //   "retention_days": 180,
    //   "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "archive_after_days": 60,
  "created_at": "2026-05-11T14:30:00Z",
  "delete_after_days": 365,
  "event_type": "user_management",
  "id": "6ba7b810-9dad-11d1-80b4-00c04fd430ce",
  "retention_days": 180,
  "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
}"###,
    ) {
        Ok(parsed) => return HttpJson::ok(parsed),
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    HttpJson::ok(Response {
        archive_after_days: Some(60),
        created_at: Some("2026-05-11T14:30:00Z".to_string()),
        delete_after_days: Some(365),
        event_type: "user_management".to_string(),
        id: Some("6ba7b810-9dad-11d1-80b4-00c04fd430ce".to_string()),
        retention_days: 180,
        tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
    })
}
