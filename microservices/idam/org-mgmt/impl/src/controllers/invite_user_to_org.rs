use brrtrouter_macros::handler;
use sesame_idam_org_mgmt_gen::handlers::invite_user_to_org::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

/// Handler for Invite User To Org.
#[handler(InviteUserToOrgController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let event = AuditEvent::new_with_params(
        AuditEventType::Organization,
        "org_invite_sent",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.org_id = req.inner.org_id.parse::<Uuid>().ok();
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.metadata = serde_json::json!({ "email": req.inner.email }).into();
    EMITTER.emit(event);

    Response {
        success: req.inner.success.unwrap_or(false),
        error: req.inner.error.clone().unwrap_or_default(),
    }
}
