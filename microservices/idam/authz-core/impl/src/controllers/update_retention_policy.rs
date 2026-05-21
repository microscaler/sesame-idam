use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::update_retention_policy::{Request, Response};

/// Handler for Update Retention Policy
#[handler(UpdateRetentionPolicyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    let retention_days = req.data.retention_days.unwrap_or(90);

    Response {
        id: Some(req.data.id),
        event_type: "".to_string(),
        retention_days,
        archive_after_days: req.data.archive_after_days,
        delete_after_days: req.data.delete_after_days,
        created_at: None,
        tenant_id: req.data.x_tenant_id,
    }
}
