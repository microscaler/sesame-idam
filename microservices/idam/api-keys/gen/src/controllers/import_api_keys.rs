// User-owned controller for handler 'import_api_keys'.

use crate::handlers::import_api_keys::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(ImportApiKeysController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "failed": 0,
    //   "imported": 1,
    //   "keys": [
    //     {
    //       "imported_at": "2024-01-17T10:00:00Z",
    //       "key_id": "550e8400-e29b-41d4-a716-446655440006",
    //       "name": "Key 1"
    //     }
    //   ]
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "failed": 0,
  "imported": 1,
  "keys": [
    {
      "imported_at": "2024-01-17T10:00:00Z",
      "key_id": "550e8400-e29b-41d4-a716-446655440006",
      "name": "Key 1"
    }
  ]
}"###,
    ) {
        Ok(parsed) => return parsed,
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    Response {
        errors: Some(vec![]),
        failed_count: Some(42),
        imported_count: Some(42),
    }
}
