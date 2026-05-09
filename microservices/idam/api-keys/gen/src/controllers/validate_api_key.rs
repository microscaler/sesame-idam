// User-owned controller for handler 'validate_api_key'.

use crate::handlers::validate_api_key::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(ValidateApiKeyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "expires_at": "2025-01-15T10:30:00Z",
    //   "key_id": "550e8400-e29b-41d4-a716-446655440003",
    //   "name": "Production API Key",
    //   "permissions": [
    //     "read",
    //     "write",
    //     "delete"
    //   ],
    //   "tenant_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
    //   "valid": true
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "expires_at": "2025-01-15T10:30:00Z",
  "key_id": "550e8400-e29b-41d4-a716-446655440003",
  "name": "Production API Key",
  "permissions": [
    "read",
    "write",
    "delete"
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
        api_key_id: Some("example".to_string()),
        expires_at: Some("2025-01-15T10:30:00Z".to_string()),
        is_expired: Some(true),
        org_id: Some("example".to_string()),
        permissions: Some(vec![
            "read".to_string(),
            "write".to_string(),
            "delete".to_string(),
        ]),
        scope_type: Some("example".to_string()),
        user_id: Some("example".to_string()),
        valid: true,
    }
}
