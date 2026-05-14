// User-owned controller for handler 'list_audit_events'.

use crate::handlers::list_audit_events::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(ListAuditEventsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "events": [
    //     {
    //       "actor": "user",
    //       "event_action": "login_success",
    //       "event_type": "authentication",
    //       "id": "550e8400-e29b-41d4-a716-446655440000",
    //       "ip_address": "203.0.113.42",
    //       "severity": "info",
    //       "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
    //       "timestamp": "2026-05-11T14:30:00Z"
    //     }
    //   ],
    //   "has_more": true,
    //   "page": 1,
    //   "page_size": 50,
    //   "total": 142
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "events": [
    {
      "actor": "user",
      "event_action": "login_success",
      "event_type": "authentication",
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "ip_address": "203.0.113.42",
      "severity": "info",
      "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
      "timestamp": "2026-05-11T14:30:00Z"
    }
  ],
  "has_more": true,
  "page": 1,
  "page_size": 50,
  "total": 142
}"###,
    ) {
        Ok(parsed) => return parsed,
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    Response {}
}
