use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::assign_principal_role::{Request, Response};

/// Assign a role to a principal within a tenant context.
///
/// Emits an `role_assigned` audit event and returns success.
///
/// TODO: In production, this validates role existence, stores the
/// assignment, invalidates cached effective permissions, and clears
/// the Redis cache for the user's principal evaluation.
#[handler(AssignPrincipalRoleController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_audit::events;
    use uuid::Uuid;

    let role_id = Uuid::new_v4();
    // Emit audit event: role assignment
    // Extract org_id as Option<Uuid> for the audit call
    let org_id: Uuid = req
        .data
        .org_id
        .as_ref()
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or_default();

    let emit_event = req.data.tenant_id.parse().and_then(|tenant_id| {
        let user_id = req.data.user_id.parse()?;
        let app_id = req.data.app_id.parse()?;
        Ok((tenant_id, org_id, user_id, app_id))
    });

    if let Ok((tenant_id, oid, user_id, app_id)) = emit_event {
        events::role_assigned(&EMITTER, tenant_id, oid, user_id, app_id, &req.data.role);
    }

    // In a production implementation, this would:
    // 1. Validate the role exists and belongs to the app/tenant
    // 2. Assign the role to the principal in the org
    // 3. Invalidate any cached effective permissions for this user
    // 4. Clear Redis cache for user's principal evaluation

    // For now, return success
    Response {
        error: String::new(),
        error_description: None,
    }
}
