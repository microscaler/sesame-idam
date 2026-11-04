// User-owned controller for handler 'validate_org_api_key'.

use crate::handlers::validate_org_api_key::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(ValidateOrgApiKeyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    // Example response:
    // {
    //   "api_key_id": "550e8400-e29b-41d4-a716-446655440003",
    //   "expires_at": 1736934600,
    //   "is_expired": false,
    //   "is_org_scoped": true,
    //   "org_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
    //   "permissions": [
    //     "read"
    //   ],
    //   "scope_type": "org",
    //   "valid": true
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "api_key_id": "550e8400-e29b-41d4-a716-446655440003",
  "expires_at": 1736934600,
  "is_expired": false,
  "is_org_scoped": true,
  "org_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
  "permissions": [
    "read"
  ],
  "scope_type": "org",
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
        is_org_scoped: true,
        org_id: Some("1189c444-8a2d-4c41-8b4b-ae43ce79a492".to_string()),
        permissions: Some(vec!["read".to_string()]),
        scope_type: Some("org".to_string()),
        user_id: Some("example".to_string()),
        valid: true,
    })
}
