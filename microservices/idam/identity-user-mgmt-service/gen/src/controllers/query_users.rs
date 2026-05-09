// User-owned controller for handler 'query_users'.

use crate::handlers::query_users::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::UserQueryItem;

#[handler(QueryUsersController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "limit": 20,
    //   "page": 1,
    //   "total": 2,
    //   "users": [
    //     {
    //       "avatar_url": null,
    //       "created_at": "2024-01-01T00:00:00Z",
    //       "email": "alice@example.com",
    //       "email_verified": true,
    //       "first_name": "Alice",
    //       "is_active": true,
    //       "last_name": "Smith",
    //       "phone": "+1234567890",
    //       "phone_verified": false,
    //       "role": "user",
    //       "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d",
    //       "username": "alice"
    //     },
    //     {
    //       "avatar_url": "https://example.com/avatars/bob.png",
    //       "created_at": "2024-01-02T00:00:00Z",
    //       "email": "bob@example.com",
    //       "email_verified": true,
    //       "first_name": "Bob",
    //       "is_active": true,
    //       "last_name": "Jones",
    //       "phone": "+1987654321",
    //       "phone_verified": true,
    //       "role": "admin",
    //       "user_id": "42d52c27-d392-55bf-0713-5b158f4cf44e",
    //       "username": "bob"
    //     }
    //   ]
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "limit": 20,
  "page": 1,
  "total": 2,
  "users": [
    {
      "avatar_url": null,
      "created_at": "2024-01-01T00:00:00Z",
      "email": "alice@example.com",
      "email_verified": true,
      "first_name": "Alice",
      "is_active": true,
      "last_name": "Smith",
      "phone": "+1234567890",
      "phone_verified": false,
      "role": "user",
      "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d",
      "username": "alice"
    },
    {
      "avatar_url": "https://example.com/avatars/bob.png",
      "created_at": "2024-01-02T00:00:00Z",
      "email": "bob@example.com",
      "email_verified": true,
      "first_name": "Bob",
      "is_active": true,
      "last_name": "Jones",
      "phone": "+1987654321",
      "phone_verified": true,
      "role": "admin",
      "user_id": "42d52c27-d392-55bf-0713-5b158f4cf44e",
      "username": "bob"
    }
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
        has_more: Some(true),limit: Some(20),page: Some(1),total: Some(2),users: Some(vec![serde_json::from_value::<UserQueryItem>(serde_json::json!({"avatar_url":null,"created_at":"2024-01-01T00:00:00Z","email":"alice@example.com","email_verified":true,"first_name":"Alice","is_active":true,"last_name":"Smith","phone":"+1234567890","phone_verified":false,"role":"user","user_id":"31c41c16-c281-44ae-9602-8a047e3bf33d","username":"alice"})).unwrap_or_default(), serde_json::from_value::<UserQueryItem>(serde_json::json!({"avatar_url":"https://example.com/avatars/bob.png","created_at":"2024-01-02T00:00:00Z","email":"bob@example.com","email_verified":true,"first_name":"Bob","is_active":true,"last_name":"Jones","phone":"+1987654321","phone_verified":true,"role":"admin","user_id":"42d52c27-d392-55bf-0713-5b158f4cf44e","username":"bob"})).unwrap_or_default()]),
    }
}
