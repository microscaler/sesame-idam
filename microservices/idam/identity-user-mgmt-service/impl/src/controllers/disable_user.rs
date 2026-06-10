use brrtrouter_macros::handler;
use sesame_idam_identity_user_mgmt_service_gen::handlers::disable_user::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

/// Handler for Disable User.
#[handler(DisableUserController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    // Span: user.disabled
    let span = tracing::span!(
        tracing::Level::INFO,
        "user.disabled",
        tenant_id = tracing::field::Empty,
        result = tracing::field::Empty
    );
    let _guard = span.enter();
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::UserManagement,
        "user_disabled",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.user_id = req.inner.user_id.parse::<Uuid>().ok();
    event.severity = Some(AuditSeverity::Warning);
    EMITTER.emit(&mut event);

    // TODO: UPDATE users SET disabled = true WHERE id = $1 AND tenant_id = $2
    // TODO: Invalidate active sessions
    
    Response {}
}
