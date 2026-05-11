// User-owned controller for handler 'fetch_fresh_oauth_token'.

use crate::handlers::fetch_fresh_oauth_token::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(FetchFreshOauthTokenController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "expires_in": 3600,
    //   "provider": "github",
    //   "token": "gho_new123abc456"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "expires_in": 3600,
  "provider": "github",
  "token": "gho_new123abc456"
}"###,
    ) {
        Ok(parsed) => return parsed,
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    Response {
        access_token: "example".to_string(),
        expires_in: Some(3600),
        refresh_token: Some("example".to_string()),
        scope: Some("example".to_string()),
        token_type: "example".to_string(),
    }
}
