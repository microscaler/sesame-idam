
// Implementation stub for handler 'create_api_key'
// This file is a starting point for your implementation.
// You can modify this file freely - it will NOT be auto-regenerated.
// To regenerate this stub, use: brrtrouter-gen generate-stubs --path create_api_key --force

use brrtrouter_macros::handler;
use sesame_idam_api_keys_gen::handlers::create_api_key::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;



#[handler(CreateApiKeyController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    // TODO: Implement your business logic here
    // 
    // Example: Access request data
    // let expires_in_days = req.inner.expires_in_days;// let metadata = req.inner.metadata;// let name = req.inner.name;// let org_id = req.inner.org_id;// let permissions = req.inner.permissions;// let user_id = req.inner.user_id;
    //
    // Example: Database query, validation, etc.
    // let result = your_service.process(&req.inner)?;
    //
    // Example: Return response
    
    Response {
        api_key: "example".to_string(), // TODO: Set from your business logic
        api_key_id: "example".to_string(), // TODO: Set from your business logic
        created_at: None, // TODO: Set from your business logic
        expires_at: None, // TODO: Set from your business logic
        name: None, // TODO: Set from your business logic
        org_id: None, // TODO: Set from your business logic
        permissions: None, // TODO: Set from your business logic
        user_id: None, // TODO: Set from your business logic
    }
    
}
