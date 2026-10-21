// User-owned controller for handler 'delete_webhook_subscription'.

use crate::handlers::delete_webhook_subscription::Request;
use brrtrouter::typed::HttpNoContent;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(DeleteWebhookSubscriptionController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpNoContent {
    HttpNoContent
}
