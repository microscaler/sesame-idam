use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::sms_magic_link_send::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

/// Handler for Sms Magic Link Send.
#[handler(SmsMagicLinkSendController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    // TODO: Look up user by phone
    // TODO: Generate signed token (JWT with exp, phone, tenant_id)
    // TODO: Store in Redis with 10min TTL
    // TODO: Send SMS via Twilio with verification code
    // TODO: Rate limit: 1 SMS per phone per 1 minute

    let mut event = AuditEvent::new(
        AuditEventType::Authentication,
        "sms_magic_link_sent",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::User,
        req.inner.ip_address.clone().unwrap_or_else(|| "127.0.0.1".to_string()),
    );
    event.metadata = serde_json::json!({ "phone": req.inner.phone }).into();
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    Response {
        magic_link_sent: true,
        expires_in: 10, // minutes
    }
}
