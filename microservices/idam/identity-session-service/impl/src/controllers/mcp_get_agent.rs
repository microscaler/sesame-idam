/// Handler for MCP Get Agent — retrieves details of an MCP agent..
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::mcp_get_agent::{Request, Response};

#[handler(McpGetAgentController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::{AuditActor, AuditEvent, AuditEventType, AuditSeverity};
    use uuid::Uuid;

    let tenant_id = _req.data.x_tenant_id.clone();

    let mut event = AuditEvent::new(
        AuditEventType::SessionManagement,
        "mcp_agent_accessed",
        tenant_id.parse::<Uuid>().unwrap_or_default(),
        AuditActor::User,
        "127.0.0.1".to_string(),
    );
    event.severity = Some(AuditSeverity::Info);
    EMITTER.emit(&mut event);

    Response {
        active: false,
        agent_id: "agent-xxx".to_string(),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        description: None,
        name: "default-agent".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    }
}
