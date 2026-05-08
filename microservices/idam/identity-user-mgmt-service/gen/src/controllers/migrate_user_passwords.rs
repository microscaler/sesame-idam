// User-owned controller for handler 'migrate_user_passwords'.

use crate::handlers::migrate_user_passwords::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(MigrateUserPasswordsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
