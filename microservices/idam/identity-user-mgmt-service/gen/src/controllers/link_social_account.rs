// User-owned controller for handler 'link_social_account'.

use crate::handlers::link_social_account::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(LinkSocialAccountController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    // Example response:
    // {
    //   "redirect_url": "https://github.com/login/oauth/authorize?client_id=abc",
    //   "state": "csrf-token-xyz"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "redirect_url": "https://github.com/login/oauth/authorize?client_id=abc",
  "state": "csrf-token-xyz"
}"###,
    ) {
        Ok(parsed) => return HttpJson::ok(parsed),
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    HttpJson::ok(Response {
        redirect_url: "https://github.com/login/oauth/authorize?client_id=abc".to_string(),
        state: "csrf-token-xyz".to_string(),
    })
}
