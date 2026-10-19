//! Tenant administrators, from a verified token only (ADR-011).
//!
//! The admin operations of authz-core (role assignments) and identity-user-mgmt-service (users) act on exactly one
//! tenant: the caller's. This module answers "who is this, and may they administer their tenant?" from the verified
//! JWT claims and nothing else. A tenant named by the caller (`X-Tenant-ID`, a body field) is never trusted: when one
//! is given it must equal the token's tenant, or the call is refused ([`TenantAdmin::require_same_tenant`]).
//!
//! # Where the claims are
//!
//! Roles and the tenant sit under the deployment's claims namespace ([`crate::jwt::claims_namespace`],
//! `SESAME_CLAIMS_NAMESPACE`); `sx` and a top-level `roles`/`tenant_id` are read too, so an older or flatter token
//! shape is neither silently a nobody nor mistaken for a grant. When the namespaced tenant and `tenant_id` disagree
//! the token is malformed and is refused.
//!
//! # Who is an administrator
//!
//! `tenant_admin` and `owner`, plus any role a deployment names in `SESAME_TENANT_ADMIN_ROLES` (comma separated; e.g.
//! a product whose staff administrator role is `platform_admin`). Role names compare case-insensitively.
use brrtrouter::typed::HttpJson;
use serde_json::Value;

/// Role that permits administering one's own tenant.
pub const ROLE_TENANT_ADMIN: &str = "tenant_admin";
/// The tenant owner can always administer it.
pub const ROLE_OWNER: &str = "owner";

/// An authenticated tenant administrator, scoped to exactly one tenant (the token's).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantAdmin {
    pub user_id: uuid::Uuid,
    pub tenant: String,
}

pub fn unauthorized(message: &str) -> HttpJson<Value> {
    HttpJson::new(
        401,
        serde_json::json!({ "error": "unauthorized", "error_description": message }),
    )
}

pub fn forbidden(message: &str) -> HttpJson<Value> {
    HttpJson::new(
        403,
        serde_json::json!({ "error": "forbidden", "error_description": message }),
    )
}

fn authz_block<'a>(claims: &'a Value, namespace: &str) -> Option<&'a Value> {
    claims.get(namespace).or_else(|| claims.get("sx"))
}

/// Role names in the token, lowercased: the namespaced (or `sx`) roles and a top-level `roles`.
pub fn roles_of(claims: &Value, namespace: &str) -> Vec<String> {
    let mut out = Vec::new();
    for source in [
        authz_block(claims, namespace).and_then(|sx| sx.get("roles")),
        claims.get("roles"),
    ] {
        if let Some(list) = source.and_then(Value::as_array) {
            out.extend(
                list.iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|r| !r.is_empty())
                    .map(str::to_ascii_lowercase),
            );
        }
    }
    out
}

/// The token's tenant: the namespaced (or `sx`) `tenant`, else `tenant_id`; None when absent or when they disagree.
pub fn tenant_of(claims: &Value, namespace: &str) -> Option<String> {
    let nested = authz_block(claims, namespace)
        .and_then(|sx| sx.get("tenant"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let flat = claims
        .get("tenant_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    match (nested, flat) {
        (Some(a), Some(b)) if a != b => None,
        (Some(a), _) => Some(a.to_string()),
        (None, Some(b)) => Some(b.to_string()),
        (None, None) => None,
    }
}

/// `tenant_admin`, `owner` and the roles listed in `extra` (comma separated, as SESAME_TENANT_ADMIN_ROLES).
pub fn admin_roles(extra: Option<&str>) -> Vec<String> {
    let mut roles = vec![ROLE_TENANT_ADMIN.to_string(), ROLE_OWNER.to_string()];
    roles.extend(
        extra
            .unwrap_or("")
            .split(',')
            .map(|r| r.trim().to_ascii_lowercase())
            .filter(|r| !r.is_empty()),
    );
    roles
}

/// The deployment's administrator roles (read once).
fn configured_admin_roles() -> &'static [String] {
    static ROLES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    ROLES.get_or_init(|| admin_roles(std::env::var("SESAME_TENANT_ADMIN_ROLES").ok().as_deref()))
}

/// Authenticate a tenant administrator from verified claims, with explicit configuration (for tests).
///
/// # Errors
/// 401 without a token, a subject or a consistent tenant; 403 when the token carries no administrator role.
pub fn tenant_admin_with(
    jwt_claims: &Option<Value>,
    namespace: &str,
    admin: &[String],
) -> Result<TenantAdmin, HttpJson<Value>> {
    let Some(claims) = jwt_claims else {
        return Err(unauthorized("Bearer token required"));
    };
    let Some(sub) = claims.get("sub").and_then(Value::as_str) else {
        return Err(unauthorized("Token missing sub claim"));
    };
    let Ok(user_id) = sub.parse::<uuid::Uuid>() else {
        return Err(unauthorized("Invalid token subject"));
    };
    let Some(tenant) = tenant_of(claims, namespace) else {
        return Err(unauthorized("Token missing or inconsistent tenant claim"));
    };
    if !roles_of(claims, namespace)
        .iter()
        .any(|r| admin.contains(r))
    {
        return Err(forbidden(
            "Tenant administrator role required for this operation",
        ));
    }
    Ok(TenantAdmin { user_id, tenant })
}

/// Authenticate a tenant administrator from verified claims (the deployment's namespace and admin roles).
///
/// # Errors
/// As [`tenant_admin_with`].
pub fn tenant_admin(jwt_claims: &Option<Value>) -> Result<TenantAdmin, HttpJson<Value>> {
    tenant_admin_with(
        jwt_claims,
        crate::jwt::claims_namespace(),
        configured_admin_roles(),
    )
}

impl TenantAdmin {
    /// A tenant the caller named (header or body) must be their own; an empty one is ignored.
    ///
    /// # Errors
    /// 403 when it names another tenant.
    pub fn require_same_tenant(&self, named: Option<&str>) -> Result<(), HttpJson<Value>> {
        match named.map(str::trim).filter(|t| !t.is_empty()) {
            Some(t) if t != self.tenant => Err(forbidden(
                "A tenant administrator acts on their own tenant only",
            )),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const NS: &str = "https://sesame-idam.example/claims";
    const SUB: &str = "11111111-1111-4111-8111-111111111111";

    fn admin() -> Vec<String> {
        admin_roles(Some("platform_admin"))
    }

    #[test]
    fn namespaced_roles_and_tenant_count() {
        let c = Some(
            json!({"sub": SUB, "tenant_id": "acme", NS: {"tenant": "acme", "roles": ["platform_admin"]}}),
        );
        let a = tenant_admin_with(&c, NS, &admin()).expect("admin");
        assert_eq!(a.tenant, "acme");
    }

    #[test]
    fn sx_and_flat_shapes_still_read() {
        let c = Some(json!({"sub": SUB, "sx": {"tenant": "acme", "roles": ["Tenant_Admin"]}}));
        assert!(tenant_admin_with(&c, NS, &admin()).is_ok());
        let c = Some(json!({"sub": SUB, "tenant_id": "acme", "roles": ["owner"]}));
        assert!(tenant_admin_with(&c, NS, &admin()).is_ok());
    }

    #[test]
    fn roles_under_another_namespace_do_not_count() {
        let c = Some(
            json!({"sub": SUB, "tenant_id": "acme", "https://elsewhere/claims": {"roles": ["owner"]}}),
        );
        assert_eq!(tenant_admin_with(&c, NS, &admin()).unwrap_err().status, 403);
    }

    #[test]
    fn ordinary_users_are_forbidden_and_anonymous_unauthorized() {
        let c = Some(
            json!({"sub": SUB, "tenant_id": "acme", NS: {"tenant": "acme", "roles": ["crm.viewer"]}}),
        );
        assert_eq!(tenant_admin_with(&c, NS, &admin()).unwrap_err().status, 403);
        assert_eq!(
            tenant_admin_with(&None, NS, &admin()).unwrap_err().status,
            401
        );
        let c = Some(json!({"tenant_id": "acme", NS: {"roles": ["owner"]}}));
        assert_eq!(tenant_admin_with(&c, NS, &admin()).unwrap_err().status, 401);
    }

    #[test]
    fn disagreeing_tenants_are_refused() {
        let c = Some(
            json!({"sub": SUB, "tenant_id": "acme", NS: {"tenant": "other", "roles": ["owner"]}}),
        );
        assert_eq!(tenant_admin_with(&c, NS, &admin()).unwrap_err().status, 401);
    }

    #[test]
    fn extra_admin_roles_are_configured_not_built_in() {
        let c = Some(
            json!({"sub": SUB, "tenant_id": "acme", NS: {"tenant": "acme", "roles": ["platform_admin"]}}),
        );
        assert!(tenant_admin_with(&c, NS, &admin_roles(None)).is_err());
    }

    #[test]
    fn a_named_tenant_must_be_the_callers() {
        let a = TenantAdmin {
            user_id: uuid::Uuid::nil(),
            tenant: "acme".into(),
        };
        assert!(a.require_same_tenant(Some("acme")).is_ok());
        assert!(a.require_same_tenant(None).is_ok());
        assert!(a.require_same_tenant(Some(" ")).is_ok());
        assert_eq!(
            a.require_same_tenant(Some("other")).unwrap_err().status,
            403
        );
    }
}
