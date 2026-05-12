use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::assign_principal_role::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(AssignPrincipalRoleController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::events;

    // Emit audit event: role assignment
    if let (Ok(tenant_id), Ok(org_id), Ok(user_id), Ok(app_id)) = (
        req.inner.tenant_id.parse(),
        req.inner.org_id.as_deref().and_then(|s| s.parse().ok()),
        req.inner.user_id.parse(),
        req.inner.app_id.parse(),
    ) {
        events::role_assigned(&EMITTER, tenant_id, org_id, user_id, app_id, &req.inner.role);
    }

    // In a production implementation, this would:
    // 1. Validate the role exists and belongs to the app/tenant
    // 2. Assign the role to the principal in the org
    // 3. Invalidate any cached effective permissions for this user
    // 4. Clear Redis cache for user's principal evaluation
    
    // For now, return success with the assigned role
    Response {
        role: Some(req.inner.role.clone()),
        user_id: req.inner.user_id,
    }
}
