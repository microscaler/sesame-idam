use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::list_audit_events::{Request, Response};

/// Handler for List Audit Events — lists audit events for the org.
#[handler(ListAuditEventsController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditActor, AuditEvent, AuditEventType, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::Compliance,
        "audit_events_listed",
        req.data.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.user_id = req.data.user_id.parse::<Uuid>().ok();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    // TODO: Query audit_events table from Postgres
    // SELECT * FROM audit_events
    // WHERE tenant_id = $1 AND event_type LIKE $2
    // ORDER BY timestamp DESC LIMIT $3 OFFSET $4

    Response {}
}
