/// Handler for MCP Get Agent — retrieves details of an MCP agent.
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_common::audit::AuditEventType;
use sesame_idam_identity_session_service_gen::handlers::mcp_get_agent::{Request, Response};

#[handler(McpGetAgentController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;

    let tenant_id = req.data.x_tenant_id.clone();

    let entry = sesame_common::audit::AuditLogEntry::new(
        AuditEventType::Delegation,
        "identity-session-service",
    )
    .tenant_id(tenant_id.clone())
    .decision_source("mcp_get_agent")
    .result("allowed")
    .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    Response {
        active: false,
        agent_id: "agent-xxx".to_string(),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        description: None,
        name: "default-agent".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    }
}
