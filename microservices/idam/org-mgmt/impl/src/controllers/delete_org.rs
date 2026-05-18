use brrtrouter_macros::handler;
use sesame_idam_org_mgmt_gen::handlers::delete_org::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(DeleteOrgController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    // Span: org.deleted
    let span = tracing::span!(
        tracing::Level::INFO,
        "org.deleted",
        tenant_id = tracing::field::Empty,
        result = tracing::field::Empty
    );
    let _guard = span.enter();
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::Organization,
        "org_deleted",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.org_id = req.inner.org_id.parse::<Uuid>().ok();
    event.severity = Some(AuditSeverity::Critical);
    EMITTER.emit(&mut event);

    Response {
        success: req.inner.success.unwrap_or(false),
        error: req.inner.error.clone().unwrap_or_default(),
    }
}
