use brrtrouter_macros::handler;
use sesame_idam_identity_user_mgmt_service_gen::handlers::delete_user::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

/// Handler for Delete User.
#[handler(DeleteUserController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    // Span: user.deleted
    let span = tracing::span!(
        tracing::Level::INFO,
        "user.deleted",
        tenant_id = tracing::field::Empty,
        result = tracing::field::Empty
    );
    let _guard = span.enter();
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::UserManagement,
        "user_deleted",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.severity = Some(AuditSeverity::Critical);
    EMITTER.emit(&mut event);

    // TODO: DELETE FROM users WHERE id = $1 AND tenant_id = $2
    // TODO: Invalidate all sessions for this user
    // TODO: Archive audit events
    
    Response {}
}
