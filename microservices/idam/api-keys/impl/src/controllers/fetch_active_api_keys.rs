
// Implementation stub for handler 'fetch_active_api_keys'
// This file is a starting point for your implementation.
// You can modify this file freely - it will NOT be auto-regenerated.
// To regenerate this stub, use: brrtrouter-gen generate-stubs --path fetch_active_api_keys --force

use brrtrouter_macros::handler;
use sesame_idam_api_keys_gen::handlers::fetch_active_api_keys::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;


#[allow(unused_imports)]
use sesame_idam_api_keys_gen::handlers::types::ApiKey;



#[handler(FetchActiveApiKeysController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    // TODO: Implement your business logic here
    // 
    // Example: Access request data
    // let user_id = req.inner.user_id;// let user_email = req.inner.user_email;// let org_id = req.inner.org_id;// let page_size = req.inner.page_size;// let page_number = req.inner.page_number;
    //
    // Example: Database query, validation, etc.
    // let result = your_service.process(&req.inner)?;
    //
    // Example: Return response
    
    Response {
        current_page: None, // TODO: Set from your business logic
        has_more_results: None, // TODO: Set from your business logic
        keys: None, // TODO: Set from your business logic
        page_size: None, // TODO: Set from your business logic
        total_keys: None, // TODO: Set from your business logic
    }
    
}
