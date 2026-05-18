use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::openid_configuration::{Request, Response};

#[handler(OpenidConfigurationController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditActor, AuditEvent, AuditEventType, AuditSeverity};
    use uuid::Uuid;

    let mut event = AuditEvent::new(
        AuditEventType::System,
        "openid_configuration_accessed",
        req.inner.tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::ServiceAccount,
        req.inner
            .ip_address
            .clone()
            .unwrap_or_else(|| "127.0.0.1".to_string()),
    );
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    Response {
        success: true,
        configuration: "{}".to_string(),
    }
}
