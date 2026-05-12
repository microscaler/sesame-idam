use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::delete_retention_policy::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(DeleteRetentionPolicyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::Compliance,
        "retention_policy_deleted",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.metadata = serde_json::json!({ "policy_id": req.inner.id }).into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    // TODO: DELETE FROM retention_policies WHERE id = $1 AND tenant_id = $2
    
    Response {}
}
