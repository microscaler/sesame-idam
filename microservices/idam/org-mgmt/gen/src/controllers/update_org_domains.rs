// User-owned controller for handler 'update_org_domains'.

use crate::handlers::update_org_domains::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(UpdateOrgDomainsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    // Example response:
    // {
    //   "error": "invalid_request",
    //   "error_description": "Bad request (validation error)"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "error": "invalid_request",
  "error_description": "Bad request (validation error)"
}"###,
    ) {
        Ok(parsed) => return HttpJson::ok(parsed),
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    HttpJson::ok(Response {
        error: "invalid_request".to_string(),
        error_description: Some("Bad request (validation error)".to_string()),
        hint: Some("example".to_string()),
        retry_after: Some(42),
    })
}
