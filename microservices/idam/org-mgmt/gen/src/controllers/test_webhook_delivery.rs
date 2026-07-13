// User-owned controller for handler 'test_webhook_delivery'.

use crate::handlers::test_webhook_delivery::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(TestWebhookDeliveryController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    HttpJson::ok(Response {
        delivery_status: Some(Default::default()),
        endpoint_url: Some("example".to_string()),
        message: Some("example".to_string()),
        success: Some(true),
    })
}
