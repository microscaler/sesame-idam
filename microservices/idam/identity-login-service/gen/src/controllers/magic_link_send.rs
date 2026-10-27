// User-owned controller for handler 'magic_link_send'.

use crate::handlers::magic_link_send::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(MagicLinkSendController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    // Example response:
    // {
    //   "message": "If the account exists, a link has been sent",
    //   "success": true
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "message": "If the account exists, a link has been sent",
  "success": true
}"###,
    ) {
        Ok(parsed) => return HttpJson::ok(parsed),
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    HttpJson::ok(Response {
        message: Some("If the account exists, a link has been sent".to_string()),
        success: true,
    })
}
