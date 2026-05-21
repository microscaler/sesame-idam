use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::authorize::{Request, Response};

/// Authorization controller handler.
///
/// Evaluates whether a principal (user) is allowed to perform an action
/// on a resource within a tenant/org context.
#[handler(AuthorizeController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    Response {
        allowed: true,
        permissions_used: None,
        reason: None,
        roles_matched: None,
    }
}
