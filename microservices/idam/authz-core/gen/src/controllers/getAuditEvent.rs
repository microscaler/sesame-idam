// User-owned controller for handler 'getAuditEvent'.

use crate::handlers::getAuditEvent::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(GetAuditEventController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "actor": "user",
    //   "event_action": "login_success",
    //   "event_type": "authentication",
    //   "id": "550e8400-e29b-41d4-a716-446655440000",
    //   "ip_address": "203.0.113.42",
    //   "session_id": "6ba7b810-9dad-11d1-80b4-00c04fd430ca",
    //   "severity": "info",
    //   "target_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c9",
    //   "target_type": "user",
    //   "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
    //   "timestamp": "2026-05-11T14:30:00Z",
    //   "user_agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
    //   "user_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c9"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "actor": "user",
  "event_action": "login_success",
  "event_type": "authentication",
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "ip_address": "203.0.113.42",
  "session_id": "6ba7b810-9dad-11d1-80b4-00c04fd430ca",
  "severity": "info",
  "target_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c9",
  "target_type": "user",
  "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
  "timestamp": "2026-05-11T14:30:00Z",
  "user_agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
  "user_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c9"
}"###,
    ) {
        Ok(parsed) => return parsed,
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    Response {
        actor: "user".to_string(),
        event_action: "login_success".to_string(),
        event_type: "authentication".to_string(),
        hmac_signature: Some("example".to_string()),
        id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        ip_address: "203.0.113.42".to_string(),
        metadata: Some(Default::default()),
        org_id: Some("example".to_string()),
        session_id: Some("6ba7b810-9dad-11d1-80b4-00c04fd430ca".to_string()),
        severity: Some("info".to_string()),
        target_id: Some("6ba7b810-9dad-11d1-80b4-00c04fd430c9".to_string()),
        target_type: Some("user".to_string()),
        tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        timestamp: "2026-05-11T14:30:00Z".to_string(),
        user_agent: Some("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)".to_string()),
        user_id: Some("6ba7b810-9dad-11d1-80b4-00c04fd430c9".to_string()),
    }
}
