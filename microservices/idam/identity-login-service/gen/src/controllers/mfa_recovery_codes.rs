// User-owned controller for handler 'mfa_recovery_codes'.

use crate::handlers::mfa_recovery_codes::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(MfaRecoveryCodesController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        recovery_codes: vec![],
    })
}
