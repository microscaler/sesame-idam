use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::list_retention_policies::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(ListRetentionPoliciesController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::Compliance,
        "retention_policies_listed",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    // TODO: SELECT * FROM retention_policies WHERE tenant_id = $1
    
    Response {
        items: vec![],
    }
}
