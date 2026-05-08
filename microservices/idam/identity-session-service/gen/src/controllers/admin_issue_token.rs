// User-owned controller for handler 'admin_issue_token'.

use crate::handlers::admin_issue_token::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(AdminIssueTokenController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        access_token: "example".to_string(),
        expires_in: 42,
        id_token: Some("example".to_string()),
        refresh_token: Some("example".to_string()),
        scope: Some("example".to_string()),
        token_type: "example".to_string(),
    }
}
