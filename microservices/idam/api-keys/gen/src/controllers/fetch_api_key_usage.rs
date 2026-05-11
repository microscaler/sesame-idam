// User-owned controller for handler 'fetch_api_key_usage'.

use crate::handlers::fetch_api_key_usage::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(FetchApiKeyUsageController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "key_id": "550e8400-e29b-41d4-a716-446655440003",
    //   "requests_last_24h": 342,
    //   "requests_last_30d": 11456,
    //   "requests_last_7d": 2891,
    //   "total_requests": 15234
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "key_id": "550e8400-e29b-41d4-a716-446655440003",
  "requests_last_24h": 342,
  "requests_last_30d": 11456,
  "requests_last_7d": 2891,
  "total_requests": 15234
}"###,
    ) {
        Ok(parsed) => return parsed,
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    Response {
        date: "example".to_string(),
        total_validations: 42,
    }
}
