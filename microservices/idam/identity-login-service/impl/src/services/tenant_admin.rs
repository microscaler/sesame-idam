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

/// Role that permits administering one's own tenant. Granted inside the tenant;
/// means nothing outside it and confers no platform capability.
pub const ROLE_TENANT_ADMIN: &str = "tenant_admin";

/// The tenant owner can always administer it. Requiring them to grant
/// themselves a second role would be a trap rather than a control.
pub const ROLE_OWNER: &str = "owner";

/// An authenticated tenant administrator, scoped to exactly one tenant.
///
/// There is deliberately no constructor that takes a tenant: the only way to
/// obtain one is [`tenant_admin_principal`], which reads the tenant from a
/// verified token.
#[derive(Debug, Clone)]
pub struct TenantAdmin {
    /// Subject of the verified token.
    pub user_id: uuid::Uuid,
    /// The one tenant this principal may act on. Taken from the token.
    pub tenant: String,
}

fn unauthorized(message: &str) -> HttpJson<Value> {
    HttpJson::new(
        401,
        serde_json::json!({ "error": "unauthorized", "error_description": message }),
    )
}

fn forbidden(message: &str) -> HttpJson<Value> {
    HttpJson::new(
        403,
        serde_json::json!({ "error": "forbidden", "error_description": message }),
    )
}

/// Collect role names from the claim shapes tokens actually use.
///
/// Roles live under the namespaced `sx` authz claim; older/flatter tokens carry
/// a top-level `roles`. Both are read so a token shape change cannot silently
/// downgrade an admin to a nobody — or, worse, be mistaken for a role grant.
fn roles_of(claims: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for source in [
        claims.get("sx").and_then(|sx| sx.get("roles")),
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

/// The tenant this token is bound to, from the claims only.
///
/// `sx.tenant` is the hard-segment isolation boundary; `tenant_id` is the
/// flatter equivalent. They must agree when both are present — a token carrying
/// two different tenants is malformed or forged, and picking one would be
/// choosing which attacker to believe.
fn tenant_of(claims: &Value) -> Option<String> {
    let sx = claims
        .get("sx")
        .and_then(|sx| sx.get("tenant"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let flat = claims
        .get("tenant_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());

    match (sx, flat) {
        (Some(a), Some(b)) if a != b => None,
        (Some(a), _) => Some(a.to_string()),
        (None, Some(b)) => Some(b.to_string()),
        (None, None) => None,
    }
}

/// Authenticate a tenant administrator from verified JWT claims.
///
/// # Errors
///
/// - 401 when there is no token, no subject, or no tenant claim.
/// - 403 when the token is valid but carries no tenant-admin role.
///
/// The distinction is deliberate: 401 means "prove who you are", 403 means "you
/// are known and this is not yours". Collapsing them would tell an attacker
/// less, but would also tell a legitimate admin nothing about why their console
/// is empty.
pub fn tenant_admin_principal(jwt_claims: &Option<Value>) -> Result<TenantAdmin, HttpJson<Value>> {
    let Some(claims) = jwt_claims else {
        return Err(unauthorized("Bearer token required"));
    };

    // A platform admin key cannot get here: this path is bearer-only, and the
    // key never produces JWT claims. That is the ADR-011 airgap — the two
    // credentials are not supersets of one another in either direction.
    let Some(sub) = claims.get("sub").and_then(Value::as_str) else {
        return Err(unauthorized("Token missing sub claim"));
    };
    let Ok(user_id) = sub.parse::<uuid::Uuid>() else {
        return Err(unauthorized("Invalid token subject"));
    };

    let Some(tenant) = tenant_of(claims) else {
        return Err(unauthorized("Token missing or inconsistent tenant claim"));
    };

    let roles = roles_of(claims);
    let is_admin = roles
        .iter()
        .any(|r| r == ROLE_TENANT_ADMIN || r == ROLE_OWNER);
    if !is_admin {
        return Err(forbidden(
            "Tenant administrator role required for this operation",
        ));
    }

    Ok(TenantAdmin { user_id, tenant })
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
        let err =
            tenant_admin_principal(&claims("acme", &["member"])).expect_err("no admin role");
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
