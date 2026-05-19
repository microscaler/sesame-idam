use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::get_audit_stats::{Request, Response};

/// Handler for Get Audit Stats — returns audit event statistics..
#[handler(GetAuditStatsController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditActor, AuditEvent, AuditEventType, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::Compliance,
        "audit_stats_requested",
        req.data.x_tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

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
