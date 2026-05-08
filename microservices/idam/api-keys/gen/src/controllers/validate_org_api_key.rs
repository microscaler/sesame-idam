// User-owned controller for handler 'validate_org_api_key'.

use crate::handlers::validate_org_api_key::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(ValidateOrgApiKeyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        is_org_scoped: true,
    }
}
