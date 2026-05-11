// User-owned controller for handler 'migrate_user'.

use crate::handlers::migrate_user::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(MigrateUserController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "email": "alice@example.com",
    //   "migrated": true,
    //   "migrated_at": "2024-01-15T10:30:00Z",
    //   "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "email": "alice@example.com",
  "migrated": true,
  "migrated_at": "2024-01-15T10:30:00Z",
  "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d"
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
        first_name: "example".to_string(),
        has_password: Some(true),
        last_name: "example".to_string(),
        locked: Some(true),
        picture_url: Some(Default::default()),
        properties: Some(Default::default()),
        user_id: "31c41c16-c281-44ae-9602-8a047e3bf33d".to_string(),
        username: "example".to_string(),
    }
}
