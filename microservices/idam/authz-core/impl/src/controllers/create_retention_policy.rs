use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::create_retention_policy::{Request, Response};

/// Handler for Create Retention Policy — creates an audit log retention policy..
#[handler(CreateRetentionPolicyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditActor, AuditEvent, AuditEventType, AuditSeverity};
    use uuid::Uuid;

    let policy_id = Uuid::new_v4();

    let mut event = AuditEvent::new(
        AuditEventType::Compliance,
        "retention_policy_created",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.metadata = serde_json::json!({
        "policy_id": policy_id.to_string(),
        "event_type": req.inner.event_type,
        "retention_days": req.inner.retention_days,
    })
    .into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    // TODO: INSERT INTO retention_policies VALUES ($1, $2, ...)

    Response {
        id: policy_id.to_string(),
        event_type: req.inner.event_type,
        retention_days: req.inner.retention_days,
        archive_after_days: req.inner.archive_after_days,
        delete_after_days: req.inner.delete_after_days,
        created_at: chrono::Utc::now().to_rfc3339(),
    }
}
