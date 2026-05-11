// User-owned controller for handler 'create_api_key'.

use crate::handlers::create_api_key::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(CreateApiKeyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "created_at": 1705312200,
    //   "expires_at": 1736934600,
    //   "key": "sk_live_abc123def456ghi789",
    //   "key_id": "550e8400-e29b-41d4-a716-446655440003",
    //   "last_used_at": null,
    //   "name": "Production API Key",
    //   "permissions": [
    //     "read",
    //     "write",
    //     "delete"
    //   ]
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "created_at": 1705312200,
  "expires_at": 1736934600,
  "key": "sk_live_abc123def456ghi789",
  "key_id": "550e8400-e29b-41d4-a716-446655440003",
  "last_used_at": null,
  "name": "Production API Key",
  "permissions": [
    "read",
    "write",
    "delete"
  ]
}"###,
    ) {
        Ok(parsed) => return parsed,
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    Response {
        api_key: "example".to_string(),
        api_key_id: "example".to_string(),
        created_at: Some(1705312200),
        expires_at: Some(1736934600),
        name: Some("Production API Key".to_string()),
        org_id: Some("example".to_string()),
        permissions: Some(vec![
            "read".to_string(),
            "write".to_string(),
            "delete".to_string(),
        ]),
        user_id: Some("example".to_string()),
    }
}
