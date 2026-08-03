// User-owned controller for handler 'challenge_org_owner_transfer'.

use crate::handlers::challenge_org_owner_transfer::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(ChallengeOrgOwnerTransferController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        channel: Some("example".to_string()),
        expires_in_secs: 42,
        success: true,
    })
}
