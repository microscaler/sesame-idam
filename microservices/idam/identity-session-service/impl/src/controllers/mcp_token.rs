/// Handler for MCP Token — issues a short-lived MCP (Model Context Protocol)
/// auth token for agent access.
// Implementation stub for handler 'mcp_token'
// Issue MCP (Model Context Protocol) auth token
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::mcp_token::{Request, Response};

#[handler(McpTokenController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    let _agent_id = req.data.agent_id.clone();
    let _credential = req.data.credential.clone();

    // TODO: Validate credential against api-keys service
    // TODO: Create MCP session
    // TODO: Issue short-lived MCP token

    Response {
        access_token: "mcp-token-xxx".to_string(),
        expires_in: None,
        scope: None,
        token_type: None,
    }
}
