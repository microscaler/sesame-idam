// User-owned controller for handler 'get_user_event_count'.

use crate::handlers::get_user_event_count::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(GetUserEventCountController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "by_type": {
    //     "authentication": 85,
    //     "session_management": 22,
    //     "user_management": 20
    //   },
    //   "time_range": {
    //     "end": "2026-05-11T23:59:59Z",
    //     "start": "2026-05-01T00:00:00Z"
    //   },
    //   "total_count": 127,
    //   "user_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c9"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "by_type": {
    "authentication": 85,
    "session_management": 22,
    "user_management": 20
  },
  "time_range": {
    "end": "2026-05-11T23:59:59Z",
    "start": "2026-05-01T00:00:00Z"
  },
  "total_count": 127,
  "user_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c9"
}"###,
    ) {
        Ok(parsed) => return parsed,
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    Response {
        by_type: Some(
            serde_json::json!({"authentication":85,"session_management":22,"user_management":20}),
        ),
        time_range: Some(
            serde_json::json!({"end":"2026-05-11T23:59:59Z","start":"2026-05-01T00:00:00Z"}),
        ),
        total_count: Some(127),
        user_id: Some("6ba7b810-9dad-11d1-80b4-00c04fd430c9".to_string()),
    }
}
