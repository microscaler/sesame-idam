use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::list_retention_policies::{Request, Response};

/// Handler for List Retention Policies — lists all retention policies.
#[handler(ListRetentionPoliciesController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // TODO: SELECT * FROM retention_policies WHERE tenant_id = $1
    Response(vec![])
}
