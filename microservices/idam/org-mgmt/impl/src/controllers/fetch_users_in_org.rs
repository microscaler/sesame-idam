
// Implementation stub for handler 'fetch_users_in_org'
// This file is a starting point for your implementation.
// You can modify this file freely - it will NOT be auto-regenerated.
// To regenerate this stub, use: brrtrouter-gen generate-stubs --path fetch_users_in_org --force

use brrtrouter_macros::handler;
use sesame_idam_org_mgmt_gen::handlers::fetch_users_in_org::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;



#[handler(FetchUsersInOrgController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    // TODO: Implement your business logic here
    // 
    // Example: Access request data
    // let org_id = req.inner.org_id;// let role = req.inner.role;// let include_orgs = req.inner.include_orgs;// let page_size = req.inner.page_size;// let page_number = req.inner.page_number;
    //
    // Example: Database query, validation, etc.
    // let result = your_service.process(&req.inner)?;
    //
    // Example: Return response
    
    Response {
        items: vec![], // TODO: Set from your business logic
        page: 42, // TODO: Set from your business logic
        page_size: 42, // TODO: Set from your business logic
        total: 42, // TODO: Set from your business logic
    }
    
}
