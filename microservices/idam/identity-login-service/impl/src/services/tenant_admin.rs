//! Tenant-administrator authority (ADR-011).
//!
//! # The rule
//!
//! In hosted SaaS mode a tenant has no association with the platform: different
//! company, different billing, different people. A tenant administrator is not
//! a junior platform administrator, so they can never be expected to hold a
//! platform credential.
//!
//! This module is the tenant half of that split. It answers one question — *who
//! is this, and which single tenant may they administer?* — and it answers it
//! **only from the verified token**.
//!
//! # Why the tenant is not a parameter
//!
//! `/tenant/*` endpoints take no tenant identifier. Not in the path, not in the
//! query, not in the body. The tenant comes from the JWT and nowhere else.
//!
//! An identifier the caller supplies is an identifier the caller can tamper
//! with, and each one needs a guard that somebody has to remember to write.
//! Removing the parameter removes the class: there is no `{slug}` to swap, so
//! cross-tenant access is not "checked and denied", it is unrepresentable. Same
//! move as the `purpose → billing owner` constant map in ADR-009, for the same
//! reason.
//!
//! If a tenant identifier ever appears in a `/tenant/*` signature, that is the
//! bug — not a missing check.

use brrtrouter::typed::HttpJson;
use serde_json::Value;

// One implementation for every Sesame service (sesame_common::tenant_admin): roles and the tenant are read from the
// deployment's claims namespace (SESAME_CLAIMS_NAMESPACE) as well as `sx` and the flat claims. This module used to read
// `sx` only, which no issued token carries: the namespaced block is where Sesame puts them, so every real tenant
// administrator was refused here (19 Oct 2026). A deployment may name more administrator roles in
// SESAME_TENANT_ADMIN_ROLES.
pub use sesame_common::tenant_admin::{TenantAdmin, ROLE_OWNER, ROLE_TENANT_ADMIN};

/// Authenticate a tenant administrator from verified JWT claims.
///
/// # Errors
///
/// - 401 when there is no token, no subject, or no tenant claim.
/// - 403 when the token is valid but carries no tenant-admin role.
///
/// The distinction is deliberate: 401 means "prove who you are", 403 means "you
/// are known and this is not yours".
pub fn tenant_admin_principal(jwt_claims: &Option<Value>) -> Result<TenantAdmin, HttpJson<Value>> {
    sesame_common::tenant_admin::tenant_admin(jwt_claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn claims(tenant: &str, roles: &[&str]) -> Option<Value> {
        Some(json!({
            "sub": "11111111-1111-4111-8111-111111111111",
            "sx": { "tenant": tenant, "roles": roles },
        }))
    }

    #[test]
    fn admin_role_resolves_to_the_token_tenant() {
        let admin =
            tenant_admin_principal(&claims("acme", &["tenant_admin"])).expect("tenant admin");
        assert_eq!(admin.tenant, "acme");
    }

    #[test]
    fn owner_implies_tenant_admin() {
        assert!(tenant_admin_principal(&claims("acme", &["owner"])).is_ok());
    }

    /// Roles are compared case-insensitively so an "Owner" from one grant path
    /// is not silently a different role from "owner" in another.
    #[test]
    fn role_match_is_case_insensitive() {
        assert!(tenant_admin_principal(&claims("acme", &["Tenant_Admin"])).is_ok());
    }

    #[test]
    fn ordinary_tenant_user_is_forbidden_not_unauthorized() {
        let err = tenant_admin_principal(&claims("acme", &["member"])).expect_err("no admin role");
        assert_eq!(err.status, 403, "known identity, not their resource");
    }

    #[test]
    fn missing_token_is_unauthorized() {
        let err = tenant_admin_principal(&None).expect_err("no token");
        assert_eq!(err.status, 401);
    }

    /// A token asserting two different tenants is not a puzzle to solve by
    /// preferring one — it is malformed or forged, and either way unusable.
    #[test]
    fn contradictory_tenant_claims_are_rejected() {
        let contradictory = Some(json!({
            "sub": "11111111-1111-4111-8111-111111111111",
            "tenant_id": "victim",
            "sx": { "tenant": "attacker", "roles": ["tenant_admin"] },
        }));
        let err = tenant_admin_principal(&contradictory).expect_err("inconsistent tenant");
        assert_eq!(err.status, 401);
    }

    #[test]
    fn agreeing_tenant_claims_are_accepted() {
        let agreeing = Some(json!({
            "sub": "11111111-1111-4111-8111-111111111111",
            "tenant_id": "acme",
            "sx": { "tenant": "acme", "roles": ["tenant_admin"] },
        }));
        assert_eq!(
            tenant_admin_principal(&agreeing).expect("ok").tenant,
            "acme"
        );
    }

    /// A role list is not a tenant grant: carrying "tenant_admin" says what you
    /// may do, never which tenant you may do it to.
    #[test]
    fn role_without_a_tenant_claim_is_unauthorized() {
        let no_tenant = Some(json!({
            "sub": "11111111-1111-4111-8111-111111111111",
            "sx": { "roles": ["tenant_admin"] },
        }));
        let err = tenant_admin_principal(&no_tenant).expect_err("no tenant");
        assert_eq!(err.status, 401);
    }
}
