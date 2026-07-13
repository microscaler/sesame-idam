// User-owned controller for handler 'enable_saml'.

use crate::handlers::enable_saml::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(EnableSamlController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    // Example response:
    // {
    //   "error": "validation_error",
    //   "message": "Request validation failed"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "error": "validation_error",
  "message": "Request validation failed"
}"###,
    ) {
        Ok(parsed) => return HttpJson::ok(parsed),
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    HttpJson::ok(Response {
        error: "validation_error".to_string(),
        error_description: Some("example".to_string()),
        hint: Some("example".to_string()),
        retry_after: Some(42),
    })
}
