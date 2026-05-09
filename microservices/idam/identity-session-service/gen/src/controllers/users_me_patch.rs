// User-owned controller for handler 'users_me_patch'.

use crate::handlers::users_me_patch::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(UsersMePatchController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "avatar_url": "https://example.com/avatars/alice_v2.png",
    //   "created_at": "2024-01-01T00:00:00Z",
    //   "email": "alice@example.com",
    //   "email_verified": true,
    //   "first_name": "Alice",
    //   "is_active": true,
    //   "last_name": "Johnson",
    //   "phone": "+1234567890",
    //   "phone_verified": false,
    //   "role": "user",
    //   "updated_at": "2024-01-16T12:00:00Z",
    //   "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d",
    //   "username": "alice"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "avatar_url": "https://example.com/avatars/alice_v2.png",
  "created_at": "2024-01-01T00:00:00Z",
  "email": "alice@example.com",
  "email_verified": true,
  "first_name": "Alice",
  "is_active": true,
  "last_name": "Johnson",
  "phone": "+1234567890",
  "phone_verified": false,
  "role": "user",
  "updated_at": "2024-01-16T12:00:00Z",
  "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d",
  "username": "alice"
}"###,
    ) {
        Ok(parsed) => return parsed,
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    Response {
        email: Some("alice@example.com".to_string()),
        email_verified: Some(true),
        first_name: Some("Alice".to_string()),
        last_name: Some("Johnson".to_string()),
        name: Some("example".to_string()),
        org_id: Some("example".to_string()),
        org_name: Some("example".to_string()),
        phone_number: Some("example".to_string()),
        phone_verified: Some(false),
        picture_url: Some("example".to_string()),
        preferred_username: Some("example".to_string()),
        properties: Some(Default::default()),
        sub: Some("example".to_string()),
        updated_at: Some("2024-01-16T12:00:00Z".to_string()),
        user_id: Some("31c41c16-c281-44ae-9602-8a047e3bf33d".to_string()),
        user_permissions: Some(vec![]),
        user_role: Some("example".to_string()),
        username: Some("alice".to_string()),
    }
}
