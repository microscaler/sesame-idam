// User-owned controller for handler 'magic_link_send'.

use crate::handlers::magic_link_send::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(MagicLinkSendController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    // Example response:
    // {
    //   "expires_in": 900,
    //   "magic_link_sent": true,
    //   "message": "A magic link has been sent to your email"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "expires_in": 900,
  "magic_link_sent": true,
  "message": "A magic link has been sent to your email"
}"###,
    ) {
        Ok(parsed) => return HttpJson::ok(parsed),
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    HttpJson::ok(Response {
        expires_in: Some(900),
        magic_link_sent: true,
    })
}
