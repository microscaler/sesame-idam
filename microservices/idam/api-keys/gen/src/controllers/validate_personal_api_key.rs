// User-owned controller for handler 'validate_personal_api_key'.

use crate::handlers::validate_personal_api_key::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(ValidatePersonalApiKeyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    // Example response:
    // {
    //   "api_key_id": "550e8400-e29b-41d4-a716-446655440003",
    //   "expires_at": 1736934600,
    //   "is_expired": false,
    //   "is_personal": true,
    //   "permissions": [
    //     "read",
    //     "write"
    //   ],
    //   "scope_type": "user",
    //   "user_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
    //   "valid": true
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "api_key_id": "550e8400-e29b-41d4-a716-446655440003",
  "expires_at": 1736934600,
  "is_expired": false,
  "is_personal": true,
  "permissions": [
    "read",
    "write"
  ],
  "scope_type": "user",
  "user_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
  "valid": true
}"###,
    ) {
        Ok(parsed) => return HttpJson::ok(parsed),
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    HttpJson::ok(Response {
        api_key_id: Some("550e8400-e29b-41d4-a716-446655440003".to_string()),
        expires_at: Some(1736934600),
        is_expired: Some(false),
        is_personal: true,
        org_id: Some("example".to_string()),
        permissions: Some(vec!["read".to_string(), "write".to_string()]),
        scope_type: Some("user".to_string()),
        user_id: Some("6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string()),
        valid: true,
    })
}
