use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::search_audit_events::{Request, Response};

/// Handler for Search Audit Events
#[handler(SearchAuditEventsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
