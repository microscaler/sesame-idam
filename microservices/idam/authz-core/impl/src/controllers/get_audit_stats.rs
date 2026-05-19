use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::get_audit_stats::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

/// Handler for Get Audit Stats — returns audit event statistics..
#[handler(GetAuditStatsController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::Compliance,
        "audit_stats_requested",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
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
        by_type: None,
        by_severity: None,
        by_actor: None,
        time_range: None,
    }
}
