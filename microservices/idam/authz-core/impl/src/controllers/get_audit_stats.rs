use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::get_audit_stats::{Request, Response};

/// Handler for Get Audit Stats
#[handler(GetAuditStatsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        total: 0,
        by_type: serde_json::json!({}),
        by_severity: serde_json::json!({}),
        by_actor: None,
        time_range: None,
    }
}
