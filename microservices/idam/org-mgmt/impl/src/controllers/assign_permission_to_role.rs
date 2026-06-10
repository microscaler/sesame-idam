use brrtrouter_macros::handler;
use sesame_idam_org_mgmt_gen::handlers::assign_permission_to_role::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

/// Handler for Assign Permission To Role.
#[handler(AssignPermissionToRoleController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEventType, AuditLevel, AuditLogEntry};
    use uuid::Uuid;

    let entry = AuditLogEntry::new(AuditEventType::Delegation, "org-mgmt")
        .tenant_id(req.inner.tenant_id.clone())
        .metadata(serde_json::json!({
            "role": req.inner.role,
            "permission": req.inner.permission,
            "org_id": req.inner.org_id,
        }))
        .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    Response {
        success: req.inner.success.unwrap_or(false),
        error: req.inner.error.clone().unwrap_or_default(),
    }
}
