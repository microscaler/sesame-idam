// User-owned controller for handler 'passkey_register'.

use crate::handlers::passkey_register::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::MfaFactor;
#[allow(unused_imports)]
use crate::handlers::types::TokenResponse;

#[handler(PasskeyRegisterController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        factor: Default::default(),
        recovery_codes: Some(vec![]),
        tokens: Some(Default::default()),
    })
}
