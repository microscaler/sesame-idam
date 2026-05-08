// User-owned controller for handler 'validate_personal_api_key'.

use crate::handlers::validate_personal_api_key::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(ValidatePersonalApiKeyController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response { is_personal: true }
}
