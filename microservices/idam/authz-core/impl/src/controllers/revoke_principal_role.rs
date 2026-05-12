use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::revoke_principal_role::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;

#[handler(RevokePrincipalRoleController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::events;

    // Emit audit event: role revocation
    if let (Ok(tenant_id), Ok(org_id), Ok(user_id), Ok(app_id)) = (
        req.inner.tenant_id.parse(),
        req.inner.org_id.as_deref().and_then(|s| s.parse().ok()),
        req.inner.user_id.parse(),
        req.inner.app_id.parse(),
    ) {
        events::role_revoked(&EMITTER, tenant_id, org_id, user_id, app_id, &req.inner.role);
    }

    // In a production implementation, this would:
    // 1. Remove the role assignment from the database
    // 2. Invalidate cached effective permissions
    // 3. Force re-evaluation on next authorization check

    Response {}
}
