// User-owned controller for handler 'subscribe_org_to_role_mapping'.

use crate::handlers::subscribe_org_to_role_mapping::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(SubscribeOrgToRoleMappingController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {})
}
