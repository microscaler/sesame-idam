// User-owned controller for handler 'signup_validate'.

use crate::handlers::signup_validate::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(SignupValidateController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    // Example response:
    // {
    //   "allowed": true,
    //   "email_exists": false,
    //   "phone_exists": false,
    //   "reason": null,
    //   "suggested_username": "alice_new"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "allowed": true,
  "email_exists": false,
  "phone_exists": false,
  "reason": null,
  "suggested_username": "alice_new"
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
        reasons: Some(vec![]),
        requires_mfa: Some(true),
    })
}
