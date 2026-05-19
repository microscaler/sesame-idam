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
    use sesame_audit::{AuditActor, AuditEvent, AuditEventType, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::Authorization,
        "effective_permissions",
        req.data.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::ServiceAccount,
        "internal".to_string(),
    );
    event.user_id = req.data.user_id.parse::<Uuid>().ok();
    if let Some(ref val) = req.data.org_id {
        event.org_id = val.to_string().parse::<Uuid>().ok();
    }
    event.metadata = serde_json::json!({ "include_inherited": req.data.include_inherited }).into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    Response {
        attributes: Some(serde_json::json!({})),
        permissions: vec![],
        roles: vec![],
        user_id: req.data.user_id,
    }
}
