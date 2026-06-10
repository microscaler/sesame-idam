use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::list_audit_events::{Request, Response};

/// Handler for List Audit Events — lists audit events for the org.
#[handler(ListAuditEventsController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEventType, AuditLogEntry};

    let entry = AuditLogEntry::new(AuditEventType::Delegation, "audit_events_listed")
        .tenant_id(&req.data.x_tenant_id)
        .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    // TODO: Query audit_events table from Postgres
    // SELECT * FROM audit_events
    // WHERE tenant_id = $1 AND event_type LIKE $2
    // ORDER BY timestamp DESC LIMIT $3 OFFSET $4

    Response {}
}
