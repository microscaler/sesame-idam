use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::delete_retention_policy::{Request, Response};

/// Handler for Delete Retention Policy
#[handler(DeleteRetentionPolicyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        error: String::new(),
        error_description: None,
    }
}
