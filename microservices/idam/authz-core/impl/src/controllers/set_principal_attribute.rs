use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::set_principal_attribute::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(SetPrincipalAttributeController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    // Emit audit event: attribute updated
    let mut event = AuditEvent::new(
        AuditEventType::Authorization,
        "attribute_updated",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.org_id = req.inner.org_id.as_deref().and_then(|s| s.parse().ok());
    event.metadata = serde_json::json!({
        "key": req.inner.key,
        "value_set": !req.inner.value.is_empty()
    }).into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    // In a production implementation, this would:
    // 1. Store the attribute in the principal's metadata table
    // 2. Invalidate cached effective permissions
    // 3. Optionally notify dependent services via webhook

    Response {}
}
