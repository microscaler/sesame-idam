// User-owned controller for handler 'migrate_org_isolated'.

use crate::handlers::migrate_org_isolated::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(MigrateOrgIsolatedController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
