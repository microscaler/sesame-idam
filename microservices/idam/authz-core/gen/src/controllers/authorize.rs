// User-owned controller for handler 'authorize'.

use crate::handlers::authorize::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(AuthorizeController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    // Example response:
    // {
    //   "allowed": true,
    //   "reason": "explicit_role_assignment"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "allowed": true,
  "reason": "explicit_role_assignment"
}"###,
    ) {
        Ok(parsed) => return HttpJson::ok(parsed),
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    HttpJson::ok(Response {
        allowed: true,
        permissions_used: Some(Default::default()),
        reason: Some("explicit_role_assignment".to_string()),
        roles_matched: Some(Default::default()),
    })
}
