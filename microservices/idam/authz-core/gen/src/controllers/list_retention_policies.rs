// User-owned controller for handler 'list_retention_policies'.

use crate::handlers::list_retention_policies::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::AuditRetentionPolicy;

#[handler(ListRetentionPoliciesController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // [
    //   {
    //     "archive_after_days": 90,
    //     "created_at": "2026-01-01T00:00:00Z",
    //     "delete_after_days": 730,
    //     "event_type": "authentication",
    //     "id": "6ba7b810-9dad-11d1-80b4-00c04fd430cc",
    //     "retention_days": 365,
    //     "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    //   },
    //   {
    //     "archive_after_days": 180,
    //     "created_at": "2026-01-01T00:00:00Z",
    //     "delete_after_days": 1095,
    //     "event_type": "authorization",
    //     "id": "6ba7b810-9dad-11d1-80b4-00c04fd430cd",
    //     "retention_days": 730,
    //     "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    //   }
    // ]
    match serde_json::from_str::<Response>(
        r###"[
  {
    "archive_after_days": 90,
    "created_at": "2026-01-01T00:00:00Z",
    "delete_after_days": 730,
    "event_type": "authentication",
    "id": "6ba7b810-9dad-11d1-80b4-00c04fd430cc",
    "retention_days": 365,
    "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
  },
  {
    "archive_after_days": 180,
    "created_at": "2026-01-01T00:00:00Z",
    "delete_after_days": 1095,
    "event_type": "authorization",
    "id": "6ba7b810-9dad-11d1-80b4-00c04fd430cd",
    "retention_days": 730,
    "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
  }
]"###,
    ) {
        Ok(parsed) => return parsed,
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    Response(vec![serde_json::from_value::<AuditRetentionPolicy>(serde_json::json!({"archive_after_days":90,"created_at":"2026-01-01T00:00:00Z","delete_after_days":730,"event_type":"authentication","id":"6ba7b810-9dad-11d1-80b4-00c04fd430cc","retention_days":365,"tenant_id":"6ba7b810-9dad-11d1-80b4-00c04fd430c8"})).unwrap_or_default(), serde_json::from_value::<AuditRetentionPolicy>(serde_json::json!({"archive_after_days":180,"created_at":"2026-01-01T00:00:00Z","delete_after_days":1095,"event_type":"authorization","id":"6ba7b810-9dad-11d1-80b4-00c04fd430cd","retention_days":730,"tenant_id":"6ba7b810-9dad-11d1-80b4-00c04fd430c8"})).unwrap_or_default()])
}
