// User-owned controller for handler 'preview_invitation'.

use crate::handlers::preview_invitation::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(PreviewInvitationController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        expired: Some(true),
        organization_name: Some("example".to_string()),
        valid: Some(true),
    }
}
