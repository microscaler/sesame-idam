// User-owned controller for handler 'create_role'.

use crate::handlers::create_role::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(CreateRoleController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    // Example response:
    // {
    //   "created_at": "2024-01-16T12:00:00Z",
    //   "description": "Can manage projects and view team members",
    //   "name": "Project Manager",
    //   "permissions": [
    //     "project:read",
    //     "project:write",
    //     "team:read"
    //   ],
    //   "role_id": "550e8400-e29b-41d4-a716-446655440010"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "created_at": "2024-01-16T12:00:00Z",
  "description": "Can manage projects and view team members",
  "name": "Project Manager",
  "permissions": [
    "project:read",
    "project:write",
    "team:read"
  ],
  "role_id": "550e8400-e29b-41d4-a716-446655440010"
}"###,
    ) {
        Ok(parsed) => return HttpJson::ok(parsed),
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    HttpJson::ok(Response {
        application_id: "example".to_string(),
        created_at: "2024-01-16T12:00:00Z".to_string(),
        description: Some("Can manage projects and view team members".to_string()),
        id: "example".to_string(),
        name: "Project Manager".to_string(),
        updated_at: Some("example".to_string()),
    })
}
