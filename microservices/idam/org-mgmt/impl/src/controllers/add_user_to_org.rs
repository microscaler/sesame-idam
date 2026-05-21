use brrtrouter_macros::handler;
use sesame_idam_org_mgmt_gen::handlers::add_user_to_org::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

/// Handler for Add User To Org.
#[handler(AddUserToOrgController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEventType, AuditLevel, AuditLogEntry};
    use uuid::Uuid;

    let entry = AuditLogEntry::new(AuditEventType::Delegation, "org-mgmt")
        .tenant_id(req.inner.tenant_id.clone())
        .metadata(serde_json::json!({
            "role": req.inner.role,
            "org_id": req.inner.org_id,
        }))
        .build();

    let entry = entry.and_then(|e| {
        Ok(e.user_id(
            req.inner.user_id
                .parse::<Uuid>()
                .ok()
                .map(|u| u.to_string())
                .unwrap_or_default(),
        )
        .build()?)
    });

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    Response {
        success: req.inner.success.unwrap_or(false),
        error: req.inner.error.clone().unwrap_or_default(),
    }
}
