use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::list_audit_events::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(ListAuditEventsController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::Compliance,
        "audit_events_listed",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.metadata = serde_json::json!({
        "limit": req.inner.limit,
        "offset": req.inner.offset,
    }).into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    // TODO: Query audit_events table from Postgres
    // SELECT * FROM audit_events
    // WHERE tenant_id = $1 AND event_type LIKE $2
    // ORDER BY timestamp DESC LIMIT $3 OFFSET $4
    
    Response {
        items: vec![],
        total: 0,
        limit: req.inner.limit,
        offset: req.inner.offset,
    }
}
