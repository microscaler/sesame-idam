// User-owned controller for handler 'fetch_webhook_subscriptions'.

use crate::handlers::fetch_webhook_subscriptions::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(FetchWebhookSubscriptionsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {}
}
