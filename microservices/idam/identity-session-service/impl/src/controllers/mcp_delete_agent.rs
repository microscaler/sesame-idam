/// Handler for MCP Delete Agent — deletes an MCP agent.
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_common::audit::AuditEventType;
use sesame_idam_identity_session_service_gen::handlers::mcp_delete_agent::{Request, Response};

#[handler(McpDeleteAgentController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;

    let tenant_id = req.data.x_tenant_id.clone();

    let entry = sesame_common::audit::AuditLogEntry::new(
        AuditEventType::Delegation,
        "identity-session-service",
    )
    .tenant_id(tenant_id.clone())
    .decision_source("mcp_delete_agent")
    .result("allowed")
    .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    Response {
        error: "Not implemented".to_string(),
        error_description: None,
        hint: None,
        retry_after: None,
    }
}
