// User-owned controller for handler 'validate_org_api_key'.

use crate::handlers::validate_org_api_key::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(ValidateOrgApiKeyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "expires_at": "2025-01-15T10:30:00Z",
    //   "key_id": "550e8400-e29b-41d4-a716-446655440003",
    //   "name": "Organization API Key",
    //   "permissions": [
    //     "read"
    //   ],
    //   "tenant_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
    //   "valid": true
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "expires_at": "2025-01-15T10:30:00Z",
  "key_id": "550e8400-e29b-41d4-a716-446655440003",
  "name": "Organization API Key",
  "permissions": [
    "read"
  ],
  "tenant_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
  "valid": true
}"###,
    ) {
        Ok(parsed) => return parsed,
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    Response {
        is_org_scoped: true,
    }
}
