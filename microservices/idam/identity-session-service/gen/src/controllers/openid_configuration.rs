// User-owned controller for handler 'openid_configuration'.

use crate::handlers::openid_configuration::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(OpenidConfigurationController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        authorization_endpoint: Some("example".to_string()),
        code_challenge_methods_supported: Some(vec![]),
        grant_types_supported: Some(vec![]),
        id_token_signing_alg_values_supported: Some(vec![]),
        issuer: Some("example".to_string()),
        jwks_uri: Some("example".to_string()),
        registration_endpoint: Some(Default::default()),
        response_modes_supported: Some(vec![]),
        response_types_supported: Some(vec![]),
        scopes_supported: Some(vec![]),
        subject_types_supported: Some(vec![]),
        token_endpoint: Some("example".to_string()),
        userinfo_encryption_alg_values_supported: Some(vec![]),
        userinfo_encryption_enc_values_supported: Some(vec![]),
        userinfo_endpoint: Some("example".to_string()),
        userinfo_signing_alg_values_supported: Some(vec![]),
    })
}
