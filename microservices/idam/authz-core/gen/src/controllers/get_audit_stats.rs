// User-owned controller for handler 'get_audit_stats'.

use crate::handlers::get_audit_stats::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(GetAuditStatsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    // Example response:
    // {
    //   "by_actor": {
    //     "admin": 30,
    //     "system": 12,
    //     "user": 100
    //   },
    //   "by_severity": {
    //     "critical": 0,
    //     "error": 4,
    //     "info": 120,
    //     "warning": 18
    //   },
    //   "by_type": {
    //     "authentication": 85,
    //     "authorization": 30,
    //     "session_management": 12,
    //     "user_management": 15
    //   },
    //   "time_range": {
    //     "earliest": "2026-05-01T00:00:00Z",
    //     "latest": "2026-05-11T14:30:00Z"
    //   },
    //   "total": 142
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "by_actor": {
    "admin": 30,
    "system": 12,
    "user": 100
  },
  "by_severity": {
    "critical": 0,
    "error": 4,
    "info": 120,
    "warning": 18
  },
  "by_type": {
    "authentication": 85,
    "authorization": 30,
    "session_management": 12,
    "user_management": 15
  },
  "time_range": {
    "earliest": "2026-05-01T00:00:00Z",
    "latest": "2026-05-11T14:30:00Z"
  },
  "total": 142
}"###,
    ) {
        Ok(parsed) => return HttpJson::ok(parsed),
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    HttpJson::ok(Response {
        by_actor: Some(serde_json::from_str::<serde_json::Value>(r#"{"admin":30,"system":12,"user":100}"#).unwrap_or(serde_json::Value::Null)),by_severity: serde_json::from_str::<serde_json::Value>(r#"{"critical":0,"error":4,"info":120,"warning":18}"#).unwrap_or(serde_json::Value::Null),by_type: serde_json::from_str::<serde_json::Value>(r#"{"authentication":85,"authorization":30,"session_management":12,"user_management":15}"#).unwrap_or(serde_json::Value::Null),time_range: Some(serde_json::from_str::<serde_json::Value>(r#"{"earliest":"2026-05-01T00:00:00Z","latest":"2026-05-11T14:30:00Z"}"#).unwrap_or(serde_json::Value::Null)),total: 142,
    })
}
