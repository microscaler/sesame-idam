// User-owned controller for handler 'auth_session_code'.

use crate::handlers::auth_session_code::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(AuthSessionCodeController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    // Example response:
    // {
    //   "code": "7Yk2t0Yr8Qn1pW5xQ3vB6mZ9aL4cS8dE",
    //   "expires_in": 60
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "code": "7Yk2t0Yr8Qn1pW5xQ3vB6mZ9aL4cS8dE",
  "expires_in": 60
}"###,
    ) {
        Ok(parsed) => return HttpJson::ok(parsed),
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    HttpJson::ok(Response {
        code: Some("7Yk2t0Yr8Qn1pW5xQ3vB6mZ9aL4cS8dE".to_string()),
        expires_in: Some(60),
    })
}
