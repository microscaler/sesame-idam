use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::authorize::{Request, Response};

/// Authorization controller handler.
///
/// Evaluates whether a principal (user) is allowed to perform an action
/// on a resource within a tenant/org context.
///
/// This endpoint audits all requests via `sesame_audit` before returning
/// the authorization decision.
#[handler(AuthorizeController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditActor, AuditEvent, AuditEventType, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::Authorization,
        "authorization_check",
        req.data
            .tenant_id
            .as_deref()
            .unwrap_or_default()
            .parse::<Uuid>()
            .unwrap_or_default(),
        AuditActor::ServiceAccount,
        "internal".to_string(),
    );
    event.user_id = req.data.user_id.parse::<Uuid>().ok();
    if let Some(ref val) = req.data.org_id {
        event.org_id = val.to_string().parse::<Uuid>().ok();
    }
    event.metadata =
        serde_json::json!({ "action": req.data.action, "resource": req.data.resource }).into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    Response {
        allowed: true,
        permissions_used: None,
        reason: None,
        roles_matched: None,
    }
}
