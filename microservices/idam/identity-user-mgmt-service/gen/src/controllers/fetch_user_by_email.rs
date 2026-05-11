// User-owned controller for handler 'fetch_user_by_email'.

use crate::handlers::fetch_user_by_email::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(FetchUserByEmailController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "avatar_url": "https://example.com/avatars/alice.png",
    //   "created_at": "2024-01-01T00:00:00Z",
    //   "email": "alice@example.com",
    //   "email_verified": true,
    //   "first_name": "Alice",
    //   "is_active": true,
    //   "last_name": "Smith",
    //   "phone": "+1234567890",
    //   "phone_verified": false,
    //   "role": "user",
    //   "updated_at": "2024-01-15T10:30:00Z",
    //   "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d",
    //   "username": "alice"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "avatar_url": "https://example.com/avatars/alice.png",
  "created_at": "2024-01-01T00:00:00Z",
  "email": "alice@example.com",
  "email_verified": true,
  "first_name": "Alice",
  "is_active": true,
  "last_name": "Smith",
  "phone": "+1234567890",
  "phone_verified": false,
  "role": "user",
  "updated_at": "2024-01-15T10:30:00Z",
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
        email: "alice@example.com".to_string(),
        email_confirmed: Some(true),
        enabled: true,
        first_name: "Alice".to_string(),
        has_password: Some(true),
        last_name: "Smith".to_string(),
        locked: Some(true),
        picture_url: Some("example".to_string()),
        properties: Some(Default::default()),
        user_id: "31c41c16-c281-44ae-9602-8a047e3bf33d".to_string(),
        username: "alice".to_string(),
    }
}
