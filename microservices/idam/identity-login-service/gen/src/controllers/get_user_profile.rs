// User-owned controller for handler 'get_user_profile'.

use crate::handlers::get_user_profile::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(GetUserProfileController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    // Example response:
    // {
    //   "email": "alice@example.com",
    //   "email_verified": true,
    //   "first_name": "Alice",
    //   "last_name": "Smith",
    //   "name": "Alice Smith",
    //   "phone_number": "+14155551234",
    //   "phone_verified": false,
    //   "preferred_username": "asmith",
    //   "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "email": "alice@example.com",
  "email_verified": true,
  "first_name": "Alice",
  "last_name": "Smith",
  "name": "Alice Smith",
  "phone_number": "+14155551234",
  "phone_verified": false,
  "preferred_username": "asmith",
  "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d"
}"###,
    ) {
        Ok(parsed) => return HttpJson::ok(parsed),
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    HttpJson::ok(Response {
        email: "alice@example.com".to_string(),
        email_verified: true,
        first_name: Some("Alice".to_string()),
        last_name: Some("Smith".to_string()),
        name: Some("Alice Smith".to_string()),
        phone_number: Some("+14155551234".to_string()),
        phone_verified: Some(false),
        preferred_username: Some("asmith".to_string()),
        user_id: "31c41c16-c281-44ae-9602-8a047e3bf33d".to_string(),
    })
}
