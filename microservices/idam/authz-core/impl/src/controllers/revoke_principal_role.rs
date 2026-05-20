use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::revoke_principal_role::{Request, Response};
use sesame_token_versioning::BumpReason;

/// Revoke a role from a principal within a tenant context.
///
/// Emits an `role_revoked` audit event and publishes a version bump
/// push invalidation event via Redis pub/sub (Story 5.4).
///
/// TODO: In production, this removes the role assignment from the database,
/// invalidates cached effective permissions, and forces re-evaluation on
/// the next authorization check.
#[handler(RevokePrincipalRoleController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::events;

    // Emit audit event: role revocation
    // Note: RevokePrincipalRoleRequest has no tenant_id field, use x_tenant_id header
    if let (Ok(tenant_id), Ok(user_id), Ok(app_id)) = (
        req.data.x_tenant_id.parse(),
        req.data.user_id.parse(),
        req.data.app_id.parse(),
    ) {
        events::role_revoked(
            &EMITTER,
            tenant_id,
            uuid::Uuid::default(), // No org_id in revoke request
            user_id,
            app_id,
            &req.data.role,
        );
    }

    // Story 5.4: Publish push invalidation event for role revocation
    if let Ok(tenant_id) = req.data.x_tenant_id.parse::<String>() {
        if let Some(publisher) = &*crate::audit::PUBLISHER {
            publisher.publish_tenant(
                &tenant_id,
                0, // version is managed by VersionStore
                BumpReason::RoleRevoked,
            );
        }
    }

    // In a production implementation, this would:
    // 1. Remove the role assignment from the database
    // 2. Invalidate cached effective permissions
    // 3. Force re-evaluation on next authorization check

    Response {
        error: String::new(),
        error_description: None,
    }
}
