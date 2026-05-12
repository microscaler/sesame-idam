use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::principal_effective::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(PrincipalEffectiveController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::Authorization,
        "effective_permissions",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::ServiceAccount,
        "internal".to_string(),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.org_id = req.inner.org_id.to_string().parse::<Uuid>().ok();
    event.metadata = serde_json::json!({ "include_inherited": req.inner.include_inherited }).into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    Response {
        attributes: None,
        permissions: vec![],
        roles: vec![],
        user_id: "".to_string(),
    }
}
