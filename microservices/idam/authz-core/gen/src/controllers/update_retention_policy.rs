// User-owned controller for handler 'update_retention_policy'.

use crate::handlers::update_retention_policy::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(UpdateRetentionPolicyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        archive_after_days: Some(42),
        created_at: Some("example".to_string()),
        delete_after_days: Some(42),
        event_type: "example".to_string(),
        id: Some("example".to_string()),
        retention_days: 42,
        tenant_id: "example".to_string(),
    }
}
