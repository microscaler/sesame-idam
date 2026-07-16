// User-owned controller for handler 'update_api_key'.

use crate::handlers::update_api_key::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(UpdateApiKeyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    // Example response:
    // {
    //   "created_at": 1705312200,
    //   "expires_at": 1736934600,
    //   "key": "sk_live_abc***",
    //   "key_id": "550e8400-e29b-41d4-a716-446655440003",
    //   "last_used_at": "2024-01-16T08:00:00Z",
    //   "name": "Updated Production Key",
    //   "permissions": [
    //     "read",
    //     "write"
    //   ],
    //   "updated_at": "2024-01-17T10:00:00Z"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "created_at": 1705312200,
  "expires_at": 1736934600,
  "key": "sk_live_abc***",
  "key_id": "550e8400-e29b-41d4-a716-446655440003",
  "last_used_at": "2024-01-16T08:00:00Z",
  "name": "Updated Production Key",
  "permissions": [
    "read",
    "write"
  ],
  "updated_at": "2024-01-17T10:00:00Z"
}"###,
    ) {
        Ok(parsed) => return HttpJson::ok(parsed),
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    HttpJson::ok(Response {
        active: true,
        api_key_id: "example".to_string(),
        created_at: 1705312200,
        expires_at: Some(1736934600),
        metadata: Some(Default::default()),
        name: "Updated Production Key".to_string(),
        org_id: Some(Default::default()),
        permissions: Some(vec!["read".to_string(), "write".to_string()]),
        user_id: Some(Default::default()),
    })
}
