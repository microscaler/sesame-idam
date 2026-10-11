// User-owned controller for handler 'parse_metadata'.

use crate::handlers::parse_metadata::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::SigningCert;

#[handler(ParseMetadataController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        entity_id: "example".to_string(),
        name_id_formats: vec![],
        signing_certs: vec![],
        sso_redirect_url: "example".to_string(),
    })
}
