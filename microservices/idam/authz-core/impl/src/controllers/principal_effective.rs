//! `POST /authz/principals/effective` — effective roles/permissions for a
//! principal within a tenant. Used by identity-login-service for JWT claim
//! enrichment (the single sanctioned cross-service dependency).

use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::principal_effective::{Request, Response};
use uuid::Uuid;

use crate::services::principal_service::PrincipalService;

/// Principal effective permissions controller.
///
/// Resolves role assignments and custom attributes for the principal from
/// `sesame_idam.role_assignments` / `sesame_idam.principal_attributes`,
/// tenant-scoped. Permissions resolution (role→permission mapping lives in
/// org-mgmt's tables) is not wired yet and returns empty.
///
/// This endpoint audits all requests via `sesame_audit`.
#[handler(PrincipalEffectiveController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEventType, AuditLogEntry};

    let mut metadata = serde_json::Map::new();
    if let Some(include) = req.data.include_inherited {
        metadata.insert("include_inherited".to_string(), serde_json::json!(include));
    }

    let entry = AuditLogEntry::new(AuditEventType::Delegation, "effective_permissions")
        .tenant_id(&req.data.tenant_id)
        .user_id(&req.data.user_id)
        .metadata(serde_json::Value::Object(metadata))
        .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    let Ok(principal_id) = req.data.user_id.parse::<Uuid>() else {
        tracing::warn!(user_id = %req.data.user_id, "principal_effective: non-uuid user_id");
        return Response {
            attributes: Some(serde_json::json!({})),
            permissions: vec![],
            roles: vec![],
            user_id: req.data.user_id,
        };
    };

    let exec = sesame_idam_database::db();

    let roles = match PrincipalService::role_assignments(&req.data.tenant_id, principal_id, exec) {
        Ok(assignments) => assignments
            .into_iter()
            .map(|a| {
                serde_json::json!({
                    "role": a.role_name,
                    "app_id": req.data.app_id,
                    "org_id": a.resource_id,
                    "inherited": false,
                })
            })
            .collect(),
        Err(e) => {
            tracing::error!(error = %e, "principal_effective: role query failed");
            vec![]
        }
    };

    let attributes = match PrincipalService::attributes(&req.data.tenant_id, principal_id, exec) {
        Ok(attrs) => {
            let map: serde_json::Map<String, serde_json::Value> = attrs
                .into_iter()
                .map(|a| (a.key, serde_json::Value::String(a.value)))
                .collect();
            Some(serde_json::Value::Object(map))
        }
        Err(e) => {
            tracing::error!(error = %e, "principal_effective: attribute query failed");
            Some(serde_json::json!({}))
        }
    };

    Response {
        attributes,
        // Role→permission mapping lives in org-mgmt's tables — resolved via
        // the entitlements snapshot work (Epic 2/7), not here yet.
        permissions: vec![],
        roles,
        user_id: req.data.user_id,
    }
}
