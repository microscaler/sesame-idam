use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::get_audit_event::{Request, Response};

/// Handler for Get Audit Event — retrieves a single audit event by ID..
#[handler(GetAuditEventController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditActor, AuditEvent, AuditEventType, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::Compliance,
        "audit_event_retrieved",
        req.data.x_tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.metadata = serde_json::json!({ "event_id": req.data.id }).into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    // TODO: Query audit_events WHERE id = $1 AND tenant_id = $2

    Response {
        id: req.data.id,
        event_type: "".to_string(),
        event_action: "".to_string(),
        actor: "".to_string(),
        ip_address: "".to_string(),
        hmac_signature: None,
        timestamp: "".to_string(),
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
