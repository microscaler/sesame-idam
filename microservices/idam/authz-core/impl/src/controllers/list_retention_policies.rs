use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::list_retention_policies::{Request, Response};

/// Handler for List Retention Policies — lists all retention policies..
#[handler(ListRetentionPoliciesController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEventType, AuditLogEntry};

    let entry = AuditLogEntry::new(AuditEventType::Delegation, "retention_policies_listed")
        .tenant_id(&req.data.x_tenant_id)
        .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    // TODO: SELECT * FROM retention_policies WHERE tenant_id = $1

    Response(vec![])
}
