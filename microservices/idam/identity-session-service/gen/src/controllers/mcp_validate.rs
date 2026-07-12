// User-owned controller for handler 'mcp_validate'.

use crate::handlers::mcp_validate::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(McpValidateController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        agent_id: Some(Default::default()),
        expires_at: Some(Default::default()),
        permissions: Some(Default::default()),
        valid: true,
    })
}
