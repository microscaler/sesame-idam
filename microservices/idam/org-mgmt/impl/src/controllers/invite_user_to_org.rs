use brrtrouter_macros::handler;
use sesame_idam_org_mgmt_gen::handlers::invite_user_to_org::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

/// Handler for Invite User To Org.
#[handler(InviteUserToOrgController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEventType, AuditLevel, AuditLogEntry};
    use uuid::Uuid;

    let entry = AuditLogEntry::new(AuditEventType::Delegation, "org-mgmt")
        .tenant_id(req.inner.tenant_id.clone())
        .metadata(serde_json::json!({
            "email": req.inner.email,
            "org_id": req.inner.org_id,
        }))
        .build();

    if let Ok(entry) = entry {
        if let Ok(uid) = req.inner.user_id.parse::<Uuid>() {
            EMITTER.emit(entry.user_id(uid.to_string()));
        } else {
            EMITTER.emit(entry);
        }
    }

    Response {
        success: req.inner.success.unwrap_or(false),
        error: req.inner.error.clone().unwrap_or_default(),
    }
}
