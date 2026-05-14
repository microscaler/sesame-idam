// User-owned controller for handler 'get_user_audit_events'.

use crate::handlers::get_user_audit_events::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(GetUserAuditEventsController)]
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
    //       "timestamp": "2026-05-11T14:30:00Z",
    //       "user_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c9"
    //     },
    //     {
    //       "actor": "user",
    //       "event_action": "mfa_enrolled",
    //       "event_type": "user_management",
    //       "id": "550e8400-e29b-41d4-a716-446655440001",
    //       "ip_address": "203.0.113.42",
    //       "severity": "info",
    //       "timestamp": "2026-05-11T14:25:00Z",
    //       "user_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c9"
    //     }
    //   ],
    //   "has_more": false,
    //   "page": 1,
    //   "page_size": 50,
    //   "total": 25
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
      "timestamp": "2026-05-11T14:30:00Z",
      "user_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c9"
    },
    {
      "actor": "user",
      "event_action": "mfa_enrolled",
      "event_type": "user_management",
      "id": "550e8400-e29b-41d4-a716-446655440001",
      "ip_address": "203.0.113.42",
      "severity": "info",
      "timestamp": "2026-05-11T14:25:00Z",
      "user_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c9"
    }
  ],
  "has_more": false,
  "page": 1,
  "page_size": 50,
  "total": 25
}"###,
    ) {
        Ok(parsed) => return parsed,
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    Response {
        events: Some(vec![
            serde_json::json!({"actor":"user","event_action":"login_success","event_type":"authentication","id":"550e8400-e29b-41d4-a716-446655440000","ip_address":"203.0.113.42","severity":"info","timestamp":"2026-05-11T14:30:00Z","user_id":"6ba7b810-9dad-11d1-80b4-00c04fd430c9"}),
            serde_json::json!({"actor":"user","event_action":"mfa_enrolled","event_type":"user_management","id":"550e8400-e29b-41d4-a716-446655440001","ip_address":"203.0.113.42","severity":"info","timestamp":"2026-05-11T14:25:00Z","user_id":"6ba7b810-9dad-11d1-80b4-00c04fd430c9"}),
        ]),
        has_more: Some(false),
        page: Some(1),
        page_size: Some(50),
        total: Some(25),
    }
}
