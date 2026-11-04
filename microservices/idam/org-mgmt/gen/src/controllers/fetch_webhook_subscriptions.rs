// User-owned controller for handler 'fetch_webhook_subscriptions'.

use crate::handlers::fetch_webhook_subscriptions::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[allow(unused_imports)]
use crate::handlers::types::WebhookSubscription;

#[handler(FetchWebhookSubscriptionsController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        has_more: true,
        page: 42,
        page_size: 42,
        subscriptions: Some(vec![]),
        total: 42,
    })
}
