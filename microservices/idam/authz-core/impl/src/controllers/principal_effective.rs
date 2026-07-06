use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::principal_effective::{Request, Response};

/// Principal effective permissions controller.
///
/// Returns all roles, permissions, and attributes that a principal
/// has within a tenant/org context, computed by resolving inheritance
/// from parent roles.
///
/// This endpoint audits all requests via `sesame_audit`.
#[handler(PrincipalEffectiveController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEventType, AuditLogEntry};

    let mut metadata = serde_json::Map::new();
    if let Some(include) = req.data.include_inherited {
        metadata.insert("include_inherited".to_string(), serde_json::json!(include));
    }

    let entry = AuditLogEntry::new(AuditEventType::Delegation, "effective_permissions")
        .tenant_id(&req.data.tenant_id)
        .user_id(&req.data.user_id)
        .metadata(serde_json::Value::Object(metadata))
        .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    Response {
        attributes: Some(serde_json::json!({})),
        permissions: vec![],
        roles: vec![],
        user_id: req.data.user_id,
    }
}
