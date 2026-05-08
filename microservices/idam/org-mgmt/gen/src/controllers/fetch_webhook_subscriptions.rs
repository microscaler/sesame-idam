// User-owned controller for handler 'fetch_webhook_subscriptions'.

use crate::handlers::fetch_webhook_subscriptions::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::WebhookSubscription;

#[handler(FetchWebhookSubscriptionsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    Response {
        subscriptions: Some(vec![]),
        total: Some(42),
    }
}
