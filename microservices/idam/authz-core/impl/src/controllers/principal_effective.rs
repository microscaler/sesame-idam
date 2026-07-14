//! `POST /authz/principals/effective` — effective roles/permissions for a
//! principal within a tenant. Used by identity-login-service for JWT claim
//! enrichment (the single sanctioned cross-service dependency).

use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::principal_effective::{Request, Response};
use uuid::Uuid;

use crate::services::principal_service::PrincipalService;

/// JSON role object for one assignment.
///
/// Tenant/application-scoped assignments have no `resource_id`; the response
/// schema types `org_id` as string, so the key is omitted rather than set to
/// `null` (a `null` fails response validation and turned the whole enrichment
/// call into a 500 — regression 2026-07-09).
fn assignment_role_json(
    role_name: &str,
    app_id: &str,
    resource_id: Option<Uuid>,
) -> serde_json::Value {
    let mut role = serde_json::json!({
        "role": role_name,
        "app_id": app_id,
        "inherited": false,
    });
    if let Some(resource_id) = resource_id {
        role["org_id"] = serde_json::json!(resource_id);
    }
    role
}

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
            .map(|a| assignment_role_json(&a.role_name, &req.data.app_id, a.resource_id))
            .collect::<Vec<_>>(),
        Err(e) => {
            tracing::error!(error = %e, "principal_effective: role query failed");
            vec![]
        }
    };

    let role_names: Vec<String> = roles
        .iter()
        .filter_map(|r| r.get("role").and_then(|v| v.as_str()).map(str::to_string))
        .collect();

    let permissions = match PrincipalService::permissions_for_roles(
        &req.data.tenant_id,
        &req.data.app_id,
        &role_names,
        exec,
    ) {
        Ok(perms) => perms,
        Err(e) => {
            tracing::error!(error = %e, "principal_effective: permission query failed");
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
        permissions,
        roles,
        user_id: req.data.user_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn org_scoped_assignment_carries_org_id() {
        let org = Uuid::parse_str("b2000002-0002-4000-8000-000000000002").unwrap();
        let role = assignment_role_json("OWNER", "hauliage", Some(org));
        assert_eq!(role["role"], "OWNER");
        assert_eq!(role["app_id"], "hauliage");
        assert_eq!(role["org_id"], org.to_string());
        assert_eq!(role["inherited"], false);
    }

    #[test]
    fn tenant_scoped_assignment_omits_org_id_key() {
        let role = assignment_role_json("ADMIN", "hauliage", None);
        // Schema types org_id as string — the key must be absent, not null.
        assert!(role.get("org_id").is_none());
        assert_eq!(role["role"], "ADMIN");
    }
}
