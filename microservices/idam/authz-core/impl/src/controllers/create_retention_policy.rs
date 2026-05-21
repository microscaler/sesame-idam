use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::create_retention_policy::{Request, Response};

/// Handler for Create Retention Policy
#[handler(CreateRetentionPolicyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use uuid::Uuid;

    let policy_id = Uuid::new_v4();

    Response {
        id: Some(policy_id.to_string()),
        event_type: req.data.event_type,
        retention_days: req.data.retention_days,
        archive_after_days: req.data.archive_after_days,
        delete_after_days: req.data.delete_after_days,
        created_at: Some(chrono::Utc::now().to_rfc3339()),
        tenant_id: req.data.x_tenant_id,
    }
}
