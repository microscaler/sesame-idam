use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::get_audit_event::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(GetAuditEventController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::Compliance,
        "audit_event_retrieved",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.metadata = serde_json::json!({ "event_id": req.inner.id }).into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    // TODO: Query audit_events WHERE id = $1 AND tenant_id = $2
    
    Response {
        id: req.inner.id,
        event_type: "".to_string(),
        event_action: "".to_string(),
        actor: "".to_string(),
        ip_address: "".to_string(),
        hmac_signature: "".to_string(),
        timestamp: "".to_string(),
        event_payload: None,
        user_id: None,
        org_id: None,
    }
}
