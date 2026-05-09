// User-owned controller for handler 'fetch_employee'.

use crate::handlers::fetch_employee::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(FetchEmployeeController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "department": "Engineering",
    //   "email": "alice@example.com",
    //   "employee_id": "EMP-001",
    //   "first_name": "Alice",
    //   "last_name": "Smith",
    //   "title": "Senior Developer",
    //   "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "department": "Engineering",
  "email": "alice@example.com",
  "employee_id": "EMP-001",
  "first_name": "Alice",
  "last_name": "Smith",
  "title": "Senior Developer",
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
        email: Some("alice@example.com".to_string()),
        first_name: Some("Alice".to_string()),
        last_name: Some("Smith".to_string()),
        org_id_to_org_info: Some(Default::default()),
        picture_url: Some("example".to_string()),
        user_id: Some("31c41c16-c281-44ae-9602-8a047e3bf33d".to_string()),
        username: Some("example".to_string()),
    }
}
