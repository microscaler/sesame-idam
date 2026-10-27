//! Platform administrators, from a verified token only.
//!
//! Some operations act on the whole deployment, not on one tenant: the signing keys (`POST /admin/jwks/revoke` in
//! identity-session-service). Their caller must be a platform administrator: the platform-admin role
//! (`SESAME_PLATFORM_ADMIN_ROLE`, default `platform_admin`) held in one of the tenants the deployment names as its
//! operator tenants (`SESAME_PLATFORM_ADMIN_TENANTS`, comma separated; PriceWhisperer's pp: `pricewhisperer`).
//!
//! The tenant matters. A role is a string that any tenant's owner may grant inside their own tenant
//! ([`crate::role_policy`]), so `platform_admin` held in another tenant proves nothing here. Without the list nobody is a
//! platform administrator: the guard fails closed. A token minted for impersonation (an `act` claim) never is one,
//! whatever roles it carries.
//!
//! Answers 401 `unauthorized` / 403 `forbidden` ([`crate::tenant_admin::unauthorized`], [`crate::tenant_admin::forbidden`]):
//! a service using this guard lists both codes in its `ErrorResponse` enum.
use brrtrouter::typed::HttpJson;
use serde_json::Value;

use crate::tenant_admin::{forbidden, roles_of, tenant_of, unauthorized};

/// The platform-admin role when `SESAME_PLATFORM_ADMIN_ROLE` is unset.
pub const DEFAULT_ROLE: &str = "platform_admin";

/// An authenticated platform administrator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformAdmin {
    pub user_id: uuid::Uuid,
    /// The operator tenant the role is held in.
    pub tenant: String,
}

/// The operator tenants named in `raw` (comma separated, as `SESAME_PLATFORM_ADMIN_TENANTS`).
#[must_use]
pub fn operator_tenants(raw: Option<&str>) -> Vec<String> {
    raw.unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect()
}

/// Authenticate a platform administrator from verified claims, with explicit configuration (for tests).
///
/// # Errors
/// 401 without a token, a subject or a consistent tenant; 403 for an impersonation token, or without the role in an
/// operator tenant (always, when no operator tenant is configured).
pub fn platform_admin_with(
    jwt_claims: &Option<Value>,
    namespace: &str,
    role: &str,
    tenants: &[String],
) -> Result<PlatformAdmin, HttpJson<Value>> {
    let Some(claims) = jwt_claims else {
        return Err(unauthorized("Bearer token required"));
    };
    let Some(sub) = claims.get("sub").and_then(Value::as_str) else {
        return Err(unauthorized("Token missing sub claim"));
    };
    let Ok(user_id) = sub.parse::<uuid::Uuid>() else {
        return Err(unauthorized("Invalid token subject"));
    };
    if claims.get("act").is_some_and(|a| !a.is_null()) {
        return Err(forbidden(
            "An impersonation token cannot act as a platform administrator",
        ));
    }
    let Some(tenant) = tenant_of(claims, namespace) else {
        return Err(unauthorized("Token missing or inconsistent tenant claim"));
    };
    let role = role.trim().to_ascii_lowercase();
    let in_operator_tenant = tenants.contains(&tenant);
    let holds_role = roles_of(claims, namespace).contains(&role);
    if !(in_operator_tenant && holds_role) {
        return Err(forbidden(
            "Platform administrator role required for this operation",
        ));
    }
    Ok(PlatformAdmin { user_id, tenant })
}

/// Authenticate a platform administrator from verified claims (the deployment's namespace, role and operator tenants,
/// read once).
///
/// # Errors
/// As [`platform_admin_with`].
pub fn platform_admin(jwt_claims: &Option<Value>) -> Result<PlatformAdmin, HttpJson<Value>> {
    static CONFIG: std::sync::OnceLock<(String, Vec<String>)> = std::sync::OnceLock::new();
    let (role, tenants) = CONFIG.get_or_init(|| {
        (
            std::env::var("SESAME_PLATFORM_ADMIN_ROLE")
                .ok()
                .filter(|r| !r.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_ROLE.to_string()),
            operator_tenants(
                std::env::var("SESAME_PLATFORM_ADMIN_TENANTS")
                    .ok()
                    .as_deref(),
            ),
        )
    });
    platform_admin_with(jwt_claims, crate::jwt::claims_namespace(), role, tenants)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const NS: &str = "https://sesame.example/claims";
    const SUB: &str = "84593c45-5378-45ec-a60c-f22adf0b8be6";

    fn ops() -> Vec<String> {
        operator_tenants(Some(" pricewhisperer , ops "))
    }

    fn check(claims: Value) -> Result<PlatformAdmin, u16> {
        platform_admin_with(&Some(claims), NS, DEFAULT_ROLE, &ops()).map_err(|r| r.status)
    }

    #[test]
    fn the_role_in_an_operator_tenant_is_a_platform_admin() {
        let a = check(json!({"sub": SUB, "tenant_id": "pricewhisperer",
                             NS: {"tenant": "pricewhisperer", "roles": ["Platform_Admin"]}}))
        .expect("platform admin");
        assert_eq!(a.tenant, "pricewhisperer");
        assert_eq!(a.user_id.to_string(), SUB);
        // a flat token shape counts too
        assert!(
            check(json!({"sub": SUB, "tenant_id": "ops", "roles": ["platform_admin"]})).is_ok()
        );
    }

    #[test]
    fn the_role_in_any_other_tenant_is_not() {
        assert_eq!(
            check(
                json!({"sub": SUB, "tenant_id": "acme", NS: {"tenant": "acme", "roles": ["platform_admin"]}})
            ),
            Err(403)
        );
    }

    #[test]
    fn a_valid_token_without_the_role_is_not() {
        for roles in [
            json!([]),
            json!(["tenant_admin", "owner"]),
            json!(["platform_admin_x"]),
        ] {
            assert_eq!(
                check(
                    json!({"sub": SUB, "tenant_id": "pricewhisperer", NS: {"tenant": "pricewhisperer", "roles": roles}})
                ),
                Err(403)
            );
        }
    }

    #[test]
    fn without_operator_tenants_nobody_is() {
        let claims = json!({"sub": SUB, "tenant_id": "pricewhisperer", NS: {"tenant": "pricewhisperer", "roles": ["platform_admin"]}});
        let none = operator_tenants(None);
        assert!(none.is_empty());
        assert_eq!(
            platform_admin_with(&Some(claims), NS, DEFAULT_ROLE, &none).map_err(|r| r.status),
            Err(403)
        );
    }

    #[test]
    fn an_impersonation_token_never_is() {
        assert_eq!(
            check(
                json!({"sub": SUB, "tenant_id": "pricewhisperer", "act": {"sub": "someone"},
                         NS: {"tenant": "pricewhisperer", "roles": ["platform_admin"]}})
            ),
            Err(403)
        );
    }

    #[test]
    fn missing_or_malformed_identity_is_unauthorized() {
        assert_eq!(
            platform_admin_with(&None, NS, DEFAULT_ROLE, &ops()).map_err(|r| r.status),
            Err(401)
        );
        assert_eq!(
            check(json!({"tenant_id": "pricewhisperer", "roles": ["platform_admin"]})),
            Err(401)
        );
        assert_eq!(
            check(
                json!({"sub": "agent:x", "tenant_id": "pricewhisperer", "roles": ["platform_admin"]})
            ),
            Err(401)
        );
        // the namespaced tenant and tenant_id disagree: malformed
        assert_eq!(
            check(
                json!({"sub": SUB, "tenant_id": "acme", NS: {"tenant": "pricewhisperer", "roles": ["platform_admin"]}})
            ),
            Err(401)
        );
    }
}
