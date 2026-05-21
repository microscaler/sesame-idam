use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::search_audit_events::{Request, Response};

/// Handler for Search Audit Events — searches audit events across the tenant..
#[handler(SearchAuditEventsController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEventType, AuditLogEntry};

    let filters = req.data.filters.as_ref();
    let mut metadata = serde_json::Map::new();
    if let Some(f) = filters {
        metadata.insert("filter_event_type".to_string(), serde_json::json!(&f.event_type));
        metadata.insert("filter_actor".to_string(), serde_json::json!(&f.actor));
        metadata.insert("filter_action".to_string(), serde_json::json!(&f.event_action));
    }

    let entry = AuditLogEntry::new(AuditEventType::Delegation, "audit_events_searched")
        .tenant_id(&req.data.tenant_id)
        .metadata(serde_json::Value::Object(metadata))
        .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    // TODO: Query audit_events with dynamic filters
    // SELECT * FROM audit_events
    // WHERE tenant_id = $1
    // AND ($2::text IS NULL OR event_type = $2)
    // ORDER BY timestamp DESC
    // LIMIT $3 OFFSET $4

    Response {}
}
