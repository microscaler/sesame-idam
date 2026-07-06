/// Handler for MCP List Agents — lists all MCP agents for the tenant.
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_common::audit::AuditEventType;
use sesame_idam_identity_session_service_gen::handlers::mcp_list_agents::{Request, Response};

#[handler(McpListAgentsController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;

    let tenant_id = req.data.x_tenant_id.clone();

    let entry = sesame_common::audit::AuditLogEntry::new(
        AuditEventType::Delegation,
        "identity-session-service",
    )
    .tenant_id(tenant_id.clone())
    .decision_source("mcp_list_agents")
    .result("allowed")
    .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    Response {}
}
