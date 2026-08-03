// User-owned controller for handler 'transfer_org_owner'.

use crate::handlers::transfer_org_owner::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(TransferOrgOwnerController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        former_owner_disposition: "example".to_string(),
        former_owner_user_id: "example".to_string(),
        org_id: "example".to_string(),
        successor_user_id: "example".to_string(),
    })
}
