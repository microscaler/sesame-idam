// User-owned controller for handler 'fetch_active_api_keys'.

use crate::handlers::fetch_active_api_keys::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(FetchActiveApiKeysController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    // Example response:
    // {
    //   "api_keys": [
    //     {
    //       "created_at": "2024-01-15T10:30:00Z",
    //       "expires_at": "2025-01-15T10:30:00Z",
    //       "key": "sk_live_abc***",
    //       "key_id": "550e8400-e29b-41d4-a716-446655440003",
    //       "last_used_at": "2024-01-16T08:00:00Z",
    //       "name": "Production API Key",
    //       "permissions": [
    //         "read",
    //         "write",
    //         "delete"
    //       ]
    //     },
    //     {
    //       "created_at": "2024-01-10T00:00:00Z",
    //       "expires_at": "2024-07-10T00:00:00Z",
    //       "key": "sk_dev_xyz***",
    //       "key_id": "550e8400-e29b-41d4-a716-446655440004",
    //       "last_used_at": "2024-01-14T12:00:00Z",
    //       "name": "Development Key",
    //       "permissions": [
    //         "read"
    //       ]
    //     }
    //   ],
    //   "limit": 20,
    //   "page": 1,
    //   "total": 2
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "api_keys": [
    {
      "created_at": "2024-01-15T10:30:00Z",
      "expires_at": "2025-01-15T10:30:00Z",
      "key": "sk_live_abc***",
      "key_id": "550e8400-e29b-41d4-a716-446655440003",
      "last_used_at": "2024-01-16T08:00:00Z",
      "name": "Production API Key",
      "permissions": [
        "read",
        "write",
        "delete"
      ]
    },
    {
      "created_at": "2024-01-10T00:00:00Z",
      "expires_at": "2024-07-10T00:00:00Z",
      "key": "sk_dev_xyz***",
      "key_id": "550e8400-e29b-41d4-a716-446655440004",
      "last_used_at": "2024-01-14T12:00:00Z",
      "name": "Development Key",
      "permissions": [
        "read"
      ]
    }
  ],
  "limit": 20,
  "page": 1,
  "total": 2
}"###,
    ) {
        Ok(parsed) => return HttpJson::ok(parsed),
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    HttpJson::ok(Response {})
}
