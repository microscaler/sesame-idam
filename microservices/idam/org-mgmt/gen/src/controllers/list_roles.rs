// User-owned controller for handler 'list_roles'.

use crate::handlers::list_roles::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::Role;

#[handler(ListRolesController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "limit": 20,
    //   "page": 1,
    //   "roles": [
    //     {
    //       "created_at": "2024-01-16T12:00:00Z",
    //       "description": "Can manage projects and view team members",
    //       "name": "Project Manager",
    //       "permissions": [
    //         "project:read",
    //         "project:write",
    //         "team:read"
    //       ],
    //       "role_id": "550e8400-e29b-41d4-a716-446655440010"
    //     },
    //     {
    //       "created_at": "2024-01-10T00:00:00Z",
    //       "description": "Read-only access",
    //       "name": "Viewer",
    //       "permissions": [
    //         "project:read"
    //       ],
    //       "role_id": "550e8400-e29b-41d4-a716-446655440011"
    //     }
    //   ],
    //   "total": 2
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "limit": 20,
  "page": 1,
  "roles": [
    {
      "created_at": "2024-01-16T12:00:00Z",
      "description": "Can manage projects and view team members",
      "name": "Project Manager",
      "permissions": [
        "project:read",
        "project:write",
        "team:read"
      ],
      "role_id": "550e8400-e29b-41d4-a716-446655440010"
    },
    {
      "created_at": "2024-01-10T00:00:00Z",
      "description": "Read-only access",
      "name": "Viewer",
      "permissions": [
        "project:read"
      ],
      "role_id": "550e8400-e29b-41d4-a716-446655440011"
    }
  ],
  "total": 2
}"###,
    ) {
        Ok(parsed) => return parsed,
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    Response {
        items: vec![],
        page: 1,
        page_size: 42,
        total: 2,
    }
}
