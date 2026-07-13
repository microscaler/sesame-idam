// User-owned controller for handler 'sms_magic_link_send'.

use crate::handlers::sms_magic_link_send::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(SmsMagicLinkSendController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    // Example response:
    // {
    //   "expires_in": 900,
    //   "magic_link_sent": true,
    //   "message": "A magic link has been sent to your phone"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "expires_in": 900,
  "magic_link_sent": true,
  "message": "A magic link has been sent to your phone"
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
