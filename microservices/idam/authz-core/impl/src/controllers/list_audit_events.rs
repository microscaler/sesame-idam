use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::list_audit_events::{Request, Response};

/// Handler for List Audit Events — lists audit events for the org.
#[handler(ListAuditEventsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // TODO: Query audit_events table from Postgres
    Response {}
}
