use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::revoke_principal_role::{Request, Response};
use sesame_common::token_versioning::BumpReason;

/// Revoke a role from a principal within a tenant context.
///
/// Emits a delegation audit event and publishes a version bump
/// push invalidation event via Redis pub/sub (Story 5.4).
///
/// TODO: In production, this removes the role assignment from the database,
/// invalidates cached effective permissions, and forces re-evaluation on
/// the next authorization check.
#[handler(RevokePrincipalRoleController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEventType, AuditLogEntry};

    // Emit audit event: role revocation
    let mut metadata = serde_json::Map::new();
    metadata.insert("role".to_string(), serde_json::json!(&req.data.role));

    let entry = AuditLogEntry::new(AuditEventType::Delegation, "role_revoked")
        .tenant_id(&req.data.x_tenant_id)
        .user_id(&req.data.user_id)
        .metadata(serde_json::Value::Object(metadata))
        .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    // Story 5.4: Publish push invalidation event for role revocation
    let tenant_id = req.data.x_tenant_id.clone();
    if let Some(publisher) = &*crate::audit::PUBLISHER {
        publisher.publish_tenant(
            &tenant_id,
            0, // version is managed by VersionStore
            BumpReason::RoleRevoked,
        );
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
