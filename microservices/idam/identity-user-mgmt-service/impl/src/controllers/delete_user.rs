use brrtrouter_macros::handler;
use sesame_idam_identity_user_mgmt_service_gen::handlers::delete_user::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

/// Handler for Delete User.
#[handler(DeleteUserController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    // Span: user.deleted
    let _span = tracing::span!(
        tracing::Level::INFO,
        "user.deleted",
        tenant_id = tracing::field::Empty,
        result = tracing::field::Empty
    );
    // Not entered (may_tracing ADR-0001): a may coroutine can resume on another thread after a yield, and a span
    // entered on one thread and exited on another leaks its context there for good (2026-10-23 memory incident).
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
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
