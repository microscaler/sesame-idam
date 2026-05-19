// Implementation stub for handler 'scim_update_user'
// Update SCIM user in org
use brrtrouter_macros::handler;
use sesame_idam_org_mgmt_service_gen::handlers::scim_update_user::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

/// Handler for Scim Update User.
#[handler(ScimUpdateUserController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    let org_id = req.inner.org_id;
    let user_id = req.inner.user_id;
    let schema = req.inner.schema.clone();
    let user_name = req.inner.user_name;
    let name = req.inner.name.clone();
    let emails = req.inner.emails.clone();
    let active = req.inner.active;
    let roles = req.inner.roles.clone();
    
    // TODO: Validate org access
    // TODO: Update user in DB
    // TODO: Update org membership roles if changed
    
    Response {
        schemas: schema,
        id: user_id,
        user_name: user_name.unwrap_or_default(),
        name: name.unwrap_or_else(|| {
            sesame_idam_org_mgmt_service_gen::handlers::scim_update_user::Name {
                given_name: None,
                family_name: None,
            }
        }),
        emails: emails.unwrap_or_default(),
        active: active.unwrap_or(true),
        roles: roles.unwrap_or_default(),
    }
}
