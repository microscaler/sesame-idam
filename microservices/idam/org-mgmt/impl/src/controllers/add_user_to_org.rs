use brrtrouter_macros::handler;
use sesame_idam_org_mgmt_gen::handlers::add_user_to_org::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(AddUserToOrgController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::Organization,
        "org_member_added",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.org_id = req.inner.org_id.parse::<Uuid>().ok();
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.metadata = serde_json::json!({ "role": req.inner.role }).into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    Response {
        success: req.inner.success.unwrap_or(false),
        error: req.inner.error.clone().unwrap_or_default(),
    }
}
