use brrtrouter_macros::handler;
use identity_session_service_service_api::handlers::openid_configuration::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(OpenidConfigurationController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::System,
        "openid_configuration_accessed",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::ServiceAccount,
        req.inner.ip_address.clone().unwrap_or_else(|| "127.0.0.1".to_string()),
    );
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    Response {
        success: true,
        configuration: "{}".to_string(),
    }
}
