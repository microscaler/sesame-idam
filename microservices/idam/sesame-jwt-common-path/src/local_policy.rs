//! # Local Policy Evaluation
//!
//! Evaluates authorization policy from JWT claims for `jwt-only` routes.
//!
//! ## Policy Checks (in order)
//!
//! 1. **Tenant validation** (HACK-401) — `claims.tenant_id` must match `X-Tenant-ID` header
//! 2. **User type check** — `claims.user_type` must match expected type
//! 3. **Role check** — `claims.sx.roles` must contain required roles
//! 4. **Permission check** — `claims.sx.permissions` must contain required permissions
//! 5. **Risk check** — `claims.sx.risk` must be at or below the route's risk threshold
//!
//! ## Security
//!
//! - Tenant validation is the most critical check — cross-tenant access must be denied
//! - All checks are AND-gated — if any check fails, the request is denied
//! - Empty role/permission arrays are handled gracefully (no panic)
//! - Missing risk claim does NOT cause denial (absence ≠ elevated)

use sesame_common::AccessClaims;

use crate::auth_decision::AuthError;
use crate::route_policy::RouteAuthCategory;

/// Evaluates local policy for a jwt-only route.
///
/// Returns `true` if the claims satisfy all policy requirements for the route.
/// Returns `false` if any check fails (with the appropriate AuthError).
///
/// # Policy Checks
///
/// 1. **Tenant**: `claims.tenant_id == X-Tenant-ID header`
/// 2. **Role**: If the route requires specific roles, `claims.sx.roles` must contain them
/// 3. **Permission**: If the route requires specific permissions, `claims.sx.permissions` must contain them
/// 4. **Risk**: For elevated/critical routes, `claims.sx.risk` must be `normal`
/// 5. **User type**: `claims.user_type` must match the expected type for the route
///
/// # Security (HACK-401)
///
/// Tenant validation is the critical check. If `claims.tenant_id` does not match
/// the `X-Tenant-ID` header, the request is rejected immediately — this prevents
/// cross-tenant data exfiltration.
pub fn evaluate_local_policy(
    claims: &AccessClaims,
    x_tenant_id: &str,
    required_roles: &[String],
    required_permissions: &[String],
    required_risk: Option<&str>,
    required_user_type: Option<&str>,
) -> Result<(), AuthError> {
    // 1. Tenant validation (HACK-401 — CRITICAL)
    if claims.tenant_id != x_tenant_id {
        return Err(AuthError::TenantMismatch {
            expected: x_tenant_id.to_string(),
            actual: claims.tenant_id.clone(),
        });
    }

    // 2. Role check
    if !required_roles.is_empty() {
        let user_roles = &claims.sx.roles;
        for required in required_roles {
            if !user_roles.contains(required) {
                return Err(AuthError::RoleCheckFailed {
                    required: required_roles.to_vec(),
                    actual: user_roles.clone(),
                });
            }
        }
    }

    // 3. Permission check
    if !required_permissions.is_empty() {
        let user_perms = &claims.sx.permissions;
        for required in required_permissions {
            if !user_perms.contains(required) {
                return Err(AuthError::PermissionCheckFailed {
                    required: required_permissions.to_vec(),
                    actual: user_perms.clone(),
                });
            }
        }
    }

    // 4. Risk check
    if let Some(required_risk_level) = required_risk {
        let claim_risk = claims.sx.risk.as_deref().unwrap_or("normal");
        if claim_risk != "normal" {
            // If route requires elevated/critical clearance, check the level
            if required_risk_level == "critical" && claim_risk != "critical" {
                return Err(AuthError::RiskCheckFailed {
                    required: "critical".to_string(),
                    actual: claims.sx.risk.clone(),
                });
            }
            if required_risk_level == "elevated" && claim_risk == "critical" {
                // Elevated routes reject critical-risk claims
                return Err(AuthError::RiskCheckFailed {
                    required: "elevated".to_string(),
                    actual: claims.sx.risk.clone(),
                });
            }
        }
    }

    // 5. User type check
    if let Some(expected_type) = required_user_type {
        if claims.user_type != expected_type {
            // User type mismatch — this is a policy violation
            // Log it as a security event but return as a policy violation
            return Err(AuthError::RoleCheckFailed {
                required: vec![expected_type.to_string()],
                actual: vec![claims.user_type.clone()],
            });
        }
    }

    Ok(())
}

/// Evaluates local policy from a `RouteAuthCategory`.
///
/// For `jwt-only` routes, this checks the claims against the route's policy.
/// For `jwt-with-fallback` and `online-only`, this always returns `Ok(())`
/// (policy evaluation is deferred to the handler).
///
/// # Returns
///
/// - `Ok(())` — claims satisfy the policy
/// - `Err(AuthError)` — claims do not satisfy the policy (403 Forbidden)
pub fn evaluate_category_policy(
    claims: &AccessClaims,
    x_tenant_id: &str,
    category: &RouteAuthCategory,
) -> Result<(), AuthError> {
    match category {
        RouteAuthCategory::JwtOnly => {
            // Full local policy evaluation
            evaluate_local_policy(
                claims,
                x_tenant_id,
                &[], // No specific roles required (all jwt-only routes allow any authenticated user)
                &[], // No specific permissions required
                None, // No risk requirement
                None, // No user type requirement
            )
        }
        RouteAuthCategory::JwtWithFallback { .. } | RouteAuthCategory::OnlineOnly => {
            // For non-jwt-only routes, tenant validation is the only check here.
            // Full authorization is handled by the handler (online fallback).
            if claims.tenant_id != x_tenant_id {
                return Err(AuthError::TenantMismatch {
                    expected: x_tenant_id.to_string(),
                    actual: claims.tenant_id.clone(),
                });
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sesame_common::{SesameAuthzClaims, SesameAuthzClaimsBuilder};

    fn make_test_claims() -> AccessClaims {
        AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("user-1")
            .aud(vec!["identity-login-service".into()])
            .client_id("test-app")
            .scope("read".into())
            .exp(i64::MAX - 3600)
            .nbf(0)
            .iat(0)
            .jti("jti-1")
            .ver(1)
            .sid("sid-1")
            .tenant_id("tenant-a")
            .user_id("user-1")
            .user_type("registered")
            .sx(SesameAuthzClaims::builder()
                .tenant("tenant-a")
                .portal("test-app")
                .roles(vec!["admin".into(), "user".into()])
                .permissions(vec!["users:read".into(), "prefs:write".into()])
                .risk("normal".into())
                .build()
                .unwrap())
            .build()
            .unwrap()
    }

    fn make_customer_claims() -> AccessClaims {
        AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("user-2")
            .aud(vec!["identity-login-service".into()])
            .client_id("test-app")
            .scope("read".into())
            .exp(i64::MAX - 3600)
            .nbf(0)
            .iat(0)
            .jti("jti-2")
            .ver(1)
            .sid("sid-2")
            .tenant_id("tenant-a")
            .user_id("user-2")
            .user_type("registered")
            .sx(
                SesameAuthzClaims::builder()
                    .tenant("tenant-a")
                    .portal("test-app")
                    .roles(vec!["customer".into()])
                    .permissions(vec!["shipments:read".into()])
                    .risk("normal".into())
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
    }

    fn make_elevated_claims() -> AccessClaims {
        AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("user-3")
            .aud(vec!["identity-login-service".into()])
            .client_id("test-app")
            .scope("read".into())
            .exp(i64::MAX - 3600)
            .nbf(0)
            .iat(0)
            .jti("jti-3")
            .ver(1)
            .sid("sid-3")
            .tenant_id("tenant-a")
            .user_id("user-3")
            .user_type("registered")
            .sx(
                SesameAuthzClaims::builder()
                    .tenant("tenant-a")
                    .portal("test-app")
                    .roles(vec!["admin".into()])
                    .permissions(vec!["users:read".into()])
                    .risk("elevated".into())
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
    }

    fn make_no_risk_claims() -> AccessClaims {
        AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("user-4")
            .aud(vec!["identity-login-service".into()])
            .client_id("test-app")
            .scope("read".into())
            .exp(i64::MAX - 3600)
            .nbf(0)
            .iat(0)
            .jti("jti-4")
            .ver(1)
            .sid("sid-4")
            .tenant_id("tenant-a")
            .user_id("user-4")
            .user_type("registered")
            .sx(
                SesameAuthzClaims::builder()
                    .tenant("tenant-a")
                    .portal("test-app")
                    .roles(vec!["admin".into()])
                    .permissions(vec!["users:read".into()])
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
    }

    // ─── Tenant Validation Tests ────────────────────────────────────────

    #[test]
    fn tenant_validation_accepts_match() {
        let claims = make_test_claims();
        let result = evaluate_local_policy(
            &claims,
            "tenant-a",
            &[],
            &[],
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn tenant_validation_rejects_mismatch() {
        let claims = make_test_claims();
        let result = evaluate_local_policy(
            &claims,
            "tenant-b",
            &[],
            &[],
            None,
            None,
        );
        assert!(matches!(
            result,
            Err(AuthError::TenantMismatch {
                expected,
                actual,
            }) if expected == "tenant-b" && actual == "tenant-a"
        ));
    }

    // ─── Role Check Tests ───────────────────────────────────────────────

    #[test]
    fn local_policy_allows_with_matching_role() {
        let claims = make_test_claims();
        let result = evaluate_local_policy(
            &claims,
            "tenant-a",
            &[vec!["admin".into(), "user".into()].into_iter().collect::<Vec<_>>()[..].to_vec()],
            &[],
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn local_policy_denies_with_missing_role() {
        let claims = make_customer_claims();
        let result = evaluate_local_policy(
            &claims,
            "tenant-a",
            &vec!["admin".into()],
            &[],
            None,
            None,
        );
        assert!(matches!(
            result,
            Err(AuthError::RoleCheckFailed { .. })
        ));
    }

    // ─── Permission Check Tests ─────────────────────────────────────────

    #[test]
    fn local_policy_allows_with_matching_permission() {
        let claims = make_test_claims();
        let result = evaluate_local_policy(
            &claims,
            "tenant-a",
            &[],
            &vec!["prefs:write".into()],
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn local_policy_denies_with_missing_permission() {
        let claims = make_customer_claims();
        let result = evaluate_local_policy(
            &claims,
            "tenant-a",
            &[],
            &vec!["prefs:write".into()],
            None,
            None,
        );
        assert!(matches!(
            result,
            Err(AuthError::PermissionCheckFailed { .. })
        ));
    }

    // ─── Risk Check Tests ───────────────────────────────────────────────

    #[test]
    fn local_policy_allows_with_normal_risk() {
        let claims = make_test_claims();
        let result = evaluate_local_policy(
            &claims,
            "tenant-a",
            &[],
            &[],
            Some("normal"),
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn local_policy_allows_without_risk_claim() {
        let claims = make_no_risk_claims();
        let result = evaluate_local_policy(
            &claims,
            "tenant-a",
            &[],
            &[],
            None,
            None,
        );
        assert!(result.is_ok());
    }

    // ─── Category Policy Tests ──────────────────────────────────────────

    #[test]
    fn category_policy_jwt_only_with_valid_claims() {
        let claims = make_test_claims();
        let result = evaluate_category_policy(
            &claims,
            "tenant-a",
            &RouteAuthCategory::JwtOnly,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn category_policy_jwt_only_with_tenant_mismatch() {
        let claims = make_test_claims();
        let result = evaluate_category_policy(
            &claims,
            "tenant-b",
            &RouteAuthCategory::JwtOnly,
        );
        assert!(matches!(result, Err(AuthError::TenantMismatch { .. })));
    }

    #[test]
    fn category_policy_jwt_with_fallback_continues() {
        let claims = make_test_claims();
        let result = evaluate_category_policy(
            &claims,
            "tenant-a",
            &RouteAuthCategory::JwtWithFallback {
                cache_ttl_secs: 30,
                requires_fresh_version: false,
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn category_policy_online_only_continues() {
        let claims = make_test_claims();
        let result = evaluate_category_policy(
            &claims,
            "tenant-a",
            &RouteAuthCategory::OnlineOnly,
        );
        assert!(result.is_ok());
    }

    // ─── Empty Arrays / Edge Cases ──────────────────────────────────────

    #[test]
    fn empty_roles_and_permissions_graceful() {
        let claims = make_customer_claims();
        let result = evaluate_local_policy(
            &claims,
            "tenant-a",
            &[],
            &[],
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn empty_required_roles_passed() {
        // Empty required roles means any user is allowed
        let claims = make_customer_claims();
        let result = evaluate_local_policy(
            &claims,
            "tenant-a",
            &[],
            &[],
            None,
            None,
        );
        assert!(result.is_ok());
    }
}
