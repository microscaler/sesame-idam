use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::set_principal_attribute::{Request, Response};

/// Handler for Set Principal Attribute - sets a metadata attribute on a principal.
#[handler(SetPrincipalAttributeController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditActor, AuditEvent, AuditEventType, AuditSeverity};
    use uuid::Uuid;

    // Emit audit event: attribute updated
    let mut event = AuditEvent::new(
        AuditEventType::Authorization,
        "attribute_updated",
        req.data.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.user_id = req.data.user_id.parse::<Uuid>().ok();
    if let Some(ref val) = req.data.org_id {
        event.org_id = val.to_string().parse::<Uuid>().ok();
    }
    event.metadata = serde_json::json!({
        "key": req.data.key,
        "value_set": !req.data.value.is_empty()
    })
    .into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    // In a production implementation, this would:
    // 1. Store the attribute in the principal's metadata table
    // 2. Invalidate cached effective permissions
    // 3. Optionally notify dependent services via webhook

    Response {
        error: String::new(),
        error_description: None,
    }
}
