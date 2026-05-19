use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::update_retention_policy::{Request, Response};

/// Handler for Update Retention Policy — updates an existing audit log retention policy..
#[handler(UpdateRetentionPolicyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditActor, AuditEvent, AuditEventType, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::Compliance,
        "retention_policy_updated",
        req.data.x_tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.metadata = serde_json::json!({
        "policy_id": req.data.id,
        "retention_days": req.data.retention_days,
    })
    .into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    // TODO: UPDATE retention_policies SET retention_days = $1, ...
    // WHERE id = $2 AND tenant_id = $3

    let retention_days = req.data.retention_days.unwrap_or(90);

    Response {
        id: Some(req.data.id),
        event_type: "".to_string(),
        retention_days,
        archive_after_days: req.data.archive_after_days,
        delete_after_days: req.data.delete_after_days,
        created_at: None,
        tenant_id: req.data.x_tenant_id,
    }
}
