
// Implementation stub for handler 'delete_webhook_subscription'
// This file is a starting point for your implementation.
// You can modify this file freely - it will NOT be auto-regenerated.
// To regenerate this stub, use: brrtrouter-gen generate-stubs --path delete_webhook_subscription --force

use brrtrouter_macros::handler;
use sesame_idam_org_mgmt_gen::handlers::delete_webhook_subscription::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;



#[handler(DeleteWebhookSubscriptionController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    // TODO: Implement your business logic here
    // 
    // Example: Access request data
    // let org_id = req.inner.org_id;// let subscription_id = req.inner.subscription_id;
    //
    // Example: Database query, validation, etc.
    // let result = your_service.process(&req.inner)?;
    //
    // Example: Return response
    
    Response {
    }
    
}
