use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::search_audit_events::{Request, Response};

/// Handler for Search Audit Events — searches audit events across the tenant..
#[handler(SearchAuditEventsController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditActor, AuditEvent, AuditEventType, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::Compliance,
        "audit_events_searched",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.metadata = serde_json::json!({
        "filter_event_type": req.inner.filters.event_type,
        "filter_actor": req.inner.filters.actor,
        "filter_action": req.inner.filters.event_action,
    })
    .into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    // TODO: Query audit_events with dynamic filters
    // SELECT * FROM audit_events
    // WHERE tenant_id = $1
    // AND ($2::text IS NULL OR event_type = $2)
    // ORDER BY timestamp DESC
    // LIMIT $3 OFFSET $4

    Response {
        items: vec![],
        total: 0,
        limit: req.inner.filters.limit,
        offset: req.inner.filters.offset,
    }
}
