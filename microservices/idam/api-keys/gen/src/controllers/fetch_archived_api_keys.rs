// User-owned controller for handler 'fetch_archived_api_keys'.

use crate::handlers::fetch_archived_api_keys::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::ArchivedApiKey;

#[handler(FetchArchivedApiKeysController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "api_keys": [
    //     {
    //       "archived_at": "2024-01-10T00:00:00Z",
    //       "archived_by": "admin@example.com",
    //       "key_id": "550e8400-e29b-41d4-a716-446655440005",
    //       "name": "Archived Key"
    //     }
    //   ],
    //   "limit": 20,
    //   "page": 1,
    //   "total": 1
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "api_keys": [
    {
      "archived_at": "2024-01-10T00:00:00Z",
      "archived_by": "admin@example.com",
      "key_id": "550e8400-e29b-41d4-a716-446655440005",
      "name": "Archived Key"
    }
  ],
  "limit": 20,
  "page": 1,
  "total": 1
}"###,
    ) {
        Ok(parsed) => return parsed,
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    Response {
        current_page: Some(42),
        has_more_results: Some(true),
        keys: vec![],
        page_size: 42,
        total_keys: 42,
    }
}
