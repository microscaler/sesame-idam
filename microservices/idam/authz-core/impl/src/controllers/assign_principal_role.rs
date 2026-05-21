use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::assign_principal_role::{Request, Response};
use sesame_token_versioning::BumpReason;

/// Assign a role to a principal within a tenant context.
///
/// Publishes a version bump push invalidation event via Redis pub/sub
/// after the role assignment (Story 5.4).
#[handler(AssignPrincipalRoleController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    // Story 5.4: Publish push invalidation event for role assignment
    if let Some(publisher) = &*crate::audit::PUBLISHER {
        publisher.publish_tenant(&req.data.tenant_id, 0, BumpReason::RoleAssigned);
    }

    Response {
        error: String::new(),
        error_description: None,
    }
}
