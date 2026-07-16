// User-owned controller for handler 'validate_personal_api_key'.

use crate::handlers::validate_personal_api_key::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(ValidatePersonalApiKeyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    // Example response:
    // {
    //   "expires_at": "2025-01-15T10:30:00Z",
    //   "key_id": "550e8400-e29b-41d4-a716-446655440003",
    //   "name": "Personal API Key",
    //   "permissions": [
    //     "read",
    //     "write"
    //   ],
    //   "tenant_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
    //   "valid": true
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "expires_at": "2025-01-15T10:30:00Z",
  "key_id": "550e8400-e29b-41d4-a716-446655440003",
  "name": "Personal API Key",
  "permissions": [
    "read",
    "write"
  ],
  "tenant_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
  "valid": true
}"###,
    ) {
        Ok(parsed) => return HttpJson::ok(parsed),
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    HttpJson::ok(Response { is_personal: true })
}
