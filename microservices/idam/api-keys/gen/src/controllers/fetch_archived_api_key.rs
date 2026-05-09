// User-owned controller for handler 'fetch_archived_api_key'.

use crate::handlers::fetch_archived_api_key::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(FetchArchivedApiKeyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "archived_at": "2024-01-10T00:00:00Z",
    //   "archived_by": "admin@example.com",
    //   "key": "sk_arc_old***",
    //   "key_id": "550e8400-e29b-41d4-a716-446655440005",
    //   "name": "Archived Key"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "archived_at": "2024-01-10T00:00:00Z",
  "archived_by": "admin@example.com",
  "key": "sk_arc_old***",
  "key_id": "550e8400-e29b-41d4-a716-446655440005",
  "name": "Archived Key"
}"###,
    ) {
        Ok(parsed) => return parsed,
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    Response {
        archived_reason: Some("example".to_string()),
        reason: Some("example".to_string()),
        revoked_at: Some(42),
        revoked_by_user_id: Some("example".to_string()),
    }
}
