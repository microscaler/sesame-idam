/// Handler for MCP Delete Agent — deletes an MCP agent..
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::mcp_delete_agent::{Request, Response};

#[handler(McpDeleteAgentController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditActor, AuditEvent, AuditEventType, AuditSeverity};
    use uuid::Uuid;

    let tenant_id = _req.data.x_tenant_id.clone();

    let mut event = AuditEvent::new(
        AuditEventType::SessionManagement,
        "mcp_agent_deleted",
        tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::User,
        "127.0.0.1".to_string(),
    );
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    Response {
        error: "Not implemented".to_string(),
        error_description: None,
        hint: None,
        retry_after: None,
    }
}
