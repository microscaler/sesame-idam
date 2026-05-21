use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::revoke_principal_role::{Request, Response};
use sesame_token_versioning::BumpReason;

/// Revoke a role from a principal within a tenant context.
///
/// Publishes a version bump push invalidation event via Redis pub/sub
/// after the role revocation (Story 5.4).
#[handler(RevokePrincipalRoleController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    // Story 5.4: Publish push invalidation event for role revocation
    if let Ok(tenant_id) = req.data.x_tenant_id.parse::<String>() {
        if let Some(publisher) = &*crate::audit::PUBLISHER {
            publisher.publish_tenant(&tenant_id, 0, BumpReason::RoleRevoked);
        }
    }

    Response {
        error: String::new(),
        error_description: None,
    }
}
