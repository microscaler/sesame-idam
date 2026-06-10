use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::get_audit_stats::{Request, Response};

/// Handler for Get Audit Stats — returns audit event statistics..
#[handler(GetAuditStatsController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEventType, AuditLogEntry};

    let entry = AuditLogEntry::new(AuditEventType::Delegation, "audit_stats_requested")
        .tenant_id(&req.data.x_tenant_id)
        .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    // TODO: Aggregate query on audit_events
    // SELECT
    //   count(*) FILTER (WHERE event_type = ...) as authentication,
    //   count(*) FILTER (WHERE event_type = ...) as authorization,
    //   count(*) FILTER (WHERE event_type = ...) as user_management,
    //   count(*) as total
    // FROM audit_events WHERE tenant_id = $1

    Response {
        total: 0,
        by_type: serde_json::json!({}),
        by_severity: serde_json::json!({}),
        by_actor: None,
        time_range: None,
    }
}
