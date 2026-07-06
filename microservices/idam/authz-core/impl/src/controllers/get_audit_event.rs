use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::get_audit_event::{Request, Response};

/// Handler for Get Audit Event — retrieves a single audit event by ID..
#[handler(GetAuditEventController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEventType, AuditLogEntry};

    let mut metadata = serde_json::Map::new();
    metadata.insert("event_id".to_string(), serde_json::json!(&req.data.id));

    let entry = AuditLogEntry::new(AuditEventType::Delegation, "audit_event_retrieved")
        .tenant_id(&req.data.x_tenant_id)
        .metadata(serde_json::Value::Object(metadata))
        .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    // TODO: Query audit_events WHERE id = $1 AND tenant_id = $2

    Response {
        id: req.data.id,
        event_type: String::new(),
        event_action: String::new(),
        actor: String::new(),
        ip_address: String::new(),
        hmac_signature: None,
        timestamp: String::new(),
        metadata: None,
        org_id: None,
        session_id: None,
        severity: None,
        target_id: None,
        target_type: None,
        tenant_id: req.data.x_tenant_id,
        user_agent: None,
        user_id: None,
    }
}
