// Implementation stub for handler 'scim_create_user'
// Create SCIM user in org
use brrtrouter_macros::handler;
use sesame_idam_org_mgmt_service_gen::handlers::scim_create_user::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

/// Handler for Scim Create User.
#[handler(ScimCreateUserController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    let org_id = req.inner.org_id;
    let schema = req.inner.schema.clone();
    let user_name = req.inner.user_name.clone();
    let name = req.inner.name.clone();
    let emails = req.inner.emails.clone();
    let active = req.inner.active.unwrap_or(true);
    let roles = req.inner.roles.clone();
    
    // TODO: Validate org access
    // TODO: Create user in DB (or fetch if exists by email)
    // TODO: Add user to org with roles
    // TODO: Return SCIM user object
    
    Response {
        schemas: schema,
        id: "new-user-uuid".to_string(),
        user_name: user_name,
        name: name,
        emails: emails.unwrap_or_default(),
        active: active,
        roles: roles,
    }
}
