use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::update_retention_policy::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(UpdateRetentionPolicyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::Compliance,
        "retention_policy_updated",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.metadata = serde_json::json!({
        "policy_id": req.inner.id,
        "retention_days": req.inner.retention_days,
    }).into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    // TODO: UPDATE retention_policies SET retention_days = $1, ...
    // WHERE id = $2 AND tenant_id = $3
    
    Response {
        id: req.inner.id,
        event_type: "".to_string(),
        retention_days: req.inner.retention_days,
        archive_after_days: req.inner.archive_after_days,
        delete_after_days: req.inner.delete_after_days,
        created_at: "".to_string(),
    }
}
