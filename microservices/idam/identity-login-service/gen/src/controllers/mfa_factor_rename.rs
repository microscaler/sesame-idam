// User-owned controller for handler 'mfa_factor_rename'.

use crate::handlers::mfa_factor_rename::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::MfaFactor;

#[handler(MfaFactorRenameController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        factors: vec![],
        passkeys_available: true,
        recovery_codes_left: 42,
        required: "example".to_string(),
        totp_available: true,
    })
}
