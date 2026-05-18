use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::admin_issue_token::{Request, Response};

#[handler(AdminIssueTokenController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    let span = tracing::span!(
        tracing::Level::INFO,
        "token.issued",
        tenant_id = tracing::field::Empty,
        user_id = tracing::field::Empty
    );
    let _guard = span.enter();

    use crate::audit::EMITTER;
    use sesame_audit::{AuditActor, AuditEvent, AuditEventType, AuditSeverity};
    use uuid::Uuid;

    let tenant_id = req.data.x_tenant_id.clone();
    let user_id = req.data.user_id.clone();

    let mut event = AuditEvent::new(
        AuditEventType::SessionManagement,
        "token_issued",
        tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::Admin,
        "internal".to_string(),
    );
    event.user_id = user_id.parse::<Uuid>().ok();
    event.severity = Some(AuditSeverity::Warning);
    EMITTER.emit(&mut event);

    span.record("tenant_id", &tenant_id);
    span.record("user_id", &user_id);
    span.record("result", "success");

    Response {
        access_token: req.data.scope.clone(),
        email: None,
        email_verified: None,
        expires_in: 3600,
        id_token: None,
        mfa_required: None,
        phone_verified: None,
        refresh_token: "default-refresh".to_string(),
        refresh_token_expires_in: None,
        scope: Some(req.data.scope.clone()),
        token_type: "Bearer".to_string(),
        user_id,
    }
}
