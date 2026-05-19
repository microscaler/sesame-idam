use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::magic_link_send::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

/// Handler for Magic Link Send.
#[handler(MagicLinkSendController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    // TODO: Look up user by email
    // TODO: Generate signed token (JWT with exp, email, tenant_id)
    // TODO: Store in Redis with 10min TTL
    // TODO: Send email with magic link: https://app.example.com/auth/verify-magic?token=xxx
    // TODO: Rate limit: 1 link per email per 1 minute

    let mut event = AuditEvent::new(
        AuditEventType::Authentication,
        "magic_link_sent",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::User,
        req.inner.ip_address.clone().unwrap_or_else(|| "127.0.0.1".to_string()),
    );
    event.metadata = serde_json::json!({ "email": req.inner.email }).into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    Response {
        magic_link_sent: true,
        expires_in: 10, // minutes
    }
}
