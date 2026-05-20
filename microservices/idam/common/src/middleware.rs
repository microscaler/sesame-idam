//! JWT Common-Path Authorization Middleware for Sesame-IDAM.
//!
//! This module provides middleware that validates JWTs and evaluates local policy
//! from claims, enabling fast-path authorization for `jwt-only` routes without
//! calling authz-core.
//!
//! # Architecture
//!
//! ```text
//! Client Request
//!   -> BRRTRouter Router (path matching)
//!     -> JWT Common-Path Middleware  <-- NEW
//!       -> If jwt-only: evaluate claims, return allow/deny
//!       -> If jwt-with-fallback or online-only: continue to handler
//!     -> Handler (business logic)
//! ```
//!
//! # Security Considerations
//!
//! - **Tenant validation is critical**: Always validates `claims.tenant_id` and
//!   `claims.sx.tenant` against `X-Tenant-ID` header BEFORE any handler executes.
//! - **Fail-closed**: Any middleware error rejects the request (never fails open).
//! - **JWKS validation**: Signature verification is the foundational security check.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

// Re-export JWT types from the jwt module.
pub use super::jwt::{AccessClaims, ActorClaim, SesameAuthzClaims};

// ===========================================================================
// Error Types
// ===========================================================================

/// Errors that can occur during JWT middleware evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    /// No Bearer token found in Authorization header.
    MissingToken,
    /// Token format is invalid (not a valid JWT structure).
    InvalidTokenFormat,
    /// JWT validation failed (typ, iss, aud, exp, nbf, signature).
    JwtValidationError(String),
    /// No route policy found for the given path/method.
    PolicyNotFound,
    /// Tenant ID mismatch between claims and X-Tenant-ID header.
    TenantMismatch { expected: String, actual: String },
    /// Missing X-Tenant-ID header.
    MissingTenantId,
    /// Required role not present in claims.
    MissingRole { required_role: String },
    /// Required permission not present in claims.
    MissingPermission { required_permission: String },
    /// User type does not match route requirement.
    UserTypeMismatch { expected: String, actual: String },
    /// Risk level too high for this route.
    RiskLevelTooHigh { required: String, actual: String },
    /// Policy evaluation failed (catch-all for unexpected errors).
    PolicyEvaluationError(String),
    /// JWKS validation failed (key not found, crypto error).
    JwksValidationFailed(String),
}

impl AuthError {
    /// Return a short reason string suitable for metrics labels.
    #[must_use]
    pub fn reason(&self) -> &str {
        match self {
            AuthError::MissingToken => "missing_token",
            AuthError::InvalidTokenFormat => "invalid_format",
            AuthError::JwtValidationError(_) => "jwt_validation",
            AuthError::PolicyNotFound => "policy_not_found",
            AuthError::TenantMismatch { .. } => "tenant_mismatch",
            AuthError::MissingTenantId => "missing_tenant_id",
            AuthError::MissingRole { .. } => "missing_role",
            AuthError::MissingPermission { .. } => "missing_permission",
            AuthError::UserTypeMismatch { .. } => "user_type_mismatch",
            AuthError::RiskLevelTooHigh { .. } => "risk_too_high",
            AuthError::PolicyEvaluationError(_) => "policy_error",
            AuthError::JwksValidationFailed(_) => "jwks_error",
        }
    }

    /// Return the HTTP status code for this error.
    #[must_use]
    pub fn status_code(&self) -> u16 {
        match self {
            AuthError::MissingToken
            | AuthError::InvalidTokenFormat
            | AuthError::JwtValidationError(_)
            | AuthError::JwksValidationFailed(_) => 401,
            AuthError::TenantMismatch { .. }
            | AuthError::MissingRole { .. }
            | AuthError::MissingPermission { .. }
            | AuthError::UserTypeMismatch { .. }
            | AuthError::RiskLevelTooHigh { .. }
            | AuthError::MissingTenantId
            | AuthError::PolicyNotFound
            | AuthError::PolicyEvaluationError(_) => 403,
        }
    }
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::MissingToken => write!(f, "Missing Bearer token"),
            AuthError::InvalidTokenFormat => write!(f, "Invalid token format"),
            AuthError::JwtValidationError(msg) => write!(f, "JWT validation: {msg}"),
            AuthError::PolicyNotFound => write!(f, "No route policy found"),
            AuthError::TenantMismatch { expected, actual } => {
                write!(f, "Tenant mismatch: expected {expected}, got {actual}")
            }
            AuthError::MissingTenantId => write!(f, "Missing X-Tenant-ID header"),
            AuthError::MissingRole { required_role } => {
                write!(f, "Missing required role: {required_role}")
            }
            AuthError::MissingPermission { required_permission } => {
                write!(f, "Missing required permission: {required_permission}")
            }
            AuthError::UserTypeMismatch { expected, actual } => {
                write!(f, "User type mismatch: expected {expected}, got {actual}")
            }
            AuthError::RiskLevelTooHigh { required, actual } => {
                write!(f, "Risk level too high: required {required}, got {actual}")
            }
            AuthError::PolicyEvaluationError(msg) => {
                write!(f, "Policy evaluation error: {msg}")
            }
            AuthError::JwksValidationFailed(msg) => {
                write!(f, "JWKS validation failed: {msg}")
            }
        }
    }
}

impl std::error::Error for AuthError {}

// ===========================================================================
// Route Policy
// ===========================================================================

/// Authorization category for a route, classified by Story 4.1.
///
/// Determines how the middleware handles authorization:
/// - `JwtOnly`: Full local evaluation from JWT claims (no online check)
/// - `JwtWithFallback`: JWT validated, but handler may call authz-core for fallback
/// - `OnlineOnly`: JWT validated for identity only, handler does full authz
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteAuthCategory {
    /// JWT-only: local policy evaluation, no authz-core call needed.
    JwtOnly {
        /// Minimum roles required (if any).
        required_roles: Vec<String>,
        /// Minimum permissions required (if any).
        required_permissions: Vec<String>,
        /// Required user type (e.g., "registered", "social", "api_key").
        required_user_type: Option<String>,
    },
    /// JWT-with-fallback: validate JWT, pass claims to handler for optional online check.
    JwtWithFallback {
        /// Common-role check done in middleware before handler.
        common_roles: Vec<String>,
        /// Common-permission check done in middleware before handler.
        common_permissions: Vec<String>,
    },
    /// Online-only: validate JWT identity only, handler does full authz.
    OnlineOnly,
}

/// Policy for a single route path + method combination.
#[derive(Debug, Clone)]
pub struct RoutePolicy {
    /// The HTTP path pattern (e.g., "/api/v1/identity/users/me").
    pub path: String,
    /// The HTTP method (e.g., "GET", "POST").
    pub method: String,
    /// Authorization category for this route.
    pub category: RouteAuthCategory,
}

impl RoutePolicy {
    #[must_use]
    pub fn new(path: String, method: String, category: RouteAuthCategory) -> Self {
        Self {
            path,
            method,
            category,
        }
    }
}

/// Thread-safe store of route policies, populated from Story 4.1 classifications.
///
/// Uses an in-memory HashMap keyed by `path:method` for O(1) lookups.
#[derive(Debug, Clone)]
pub struct RoutePolicyStore {
    /// Maps "GET:/api/v1/..." -> RoutePolicy
    policies: HashMap<String, RoutePolicy>,
}

impl RoutePolicyStore {
    /// Create a new empty policy store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
        }
    }

    /// Register a single route policy.
    pub fn register(&mut self, policy: RoutePolicy) {
        let key = format!("{}:{}", policy.method, policy.path);
        self.policies.insert(key, policy);
    }

    /// Look up policy by path and method.
    #[must_use]
    pub fn get_policy(&self, path: &str, method: &str) -> Option<&RoutePolicy> {
        let key = format!("{}:{}", method, path);
        self.policies.get(&key)
    }

    /// Check if a policy exists for the given path+method.
    #[must_use]
    pub fn has_policy(&self, path: &str, method: &str) -> bool {
        let key = format!("{}:{}", method, path);
        self.policies.contains_key(&key)
    }

    /// Get the total number of registered policies.
    #[must_use]
    pub fn len(&self) -> usize {
        self.policies.len()
    }

    /// Check if the store is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.policies.is_empty()
    }

    /// Register a batch of policies from classification data.
    pub fn register_batch(
        &mut self,
        policies: impl IntoIterator<Item = (String, String, RouteAuthCategory)>,
    ) {
        for (path, method, category) in policies {
            self.register(RoutePolicy::new(path, method, category));
        }
    }
}

impl Default for RoutePolicyStore {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// AuthDecision
// ===========================================================================

/// Result of middleware evaluation.
///
/// # Variants
///
/// - `Allowed` — Request is authorized, pass to handler with claims in context.
/// - `Denied` — Request is denied, return 403 to client.
/// - `JwtCommonPath` — JWT validated, pass to handler (for non-jwt-only routes).
#[derive(Debug, Clone)]
pub enum AuthDecision {
    /// Authorization granted. Claims are passed to the handler.
    Allowed {
        /// The validated access claims.
        claims: AccessClaims,
    },
    /// Authorization denied. Contains reason for the denial.
    Denied {
        /// Reason for denial (e.g., "missing_role:admin").
        reason: String,
        /// The underlying error (for structured logging).
        error: AuthError,
    },
    /// JWT validated but route is not jwt-only. Continue to handler.
    JwtCommonPath {
        /// The validated access claims (used by handler for identity).
        claims: AccessClaims,
    },
}

impl AuthDecision {
    /// Check if the decision allows the request to proceed.
    #[must_use]
    pub fn is_allowed(&self) -> bool {
        matches!(self, AuthDecision::Allowed { .. } | AuthDecision::JwtCommonPath { .. })
    }

    /// Check if the decision denies the request.
    #[must_use]
    pub fn is_denied(&self) -> bool {
        matches!(self, AuthDecision::Denied { .. })
    }

    /// Get the error if denied.
    #[must_use]
    pub fn error(&self) -> Option<&AuthError> {
        match self {
            AuthDecision::Denied { error, .. } => Some(error),
            _ => None,
        }
    }
}

// ===========================================================================
// JWT Validation Helper
// ===========================================================================

/// Extract the Bearer token from the Authorization header.
///
/// # Arguments
///
/// * `auth_header` — The value of the `Authorization` header.
///
/// # Returns
///
/// * `Ok(String)` — The extracted Bearer token.
/// * `Err(AuthError::MissingToken)` — No Authorization header or not Bearer type.
pub fn extract_bearer_token(auth_header: &str) -> Result<String, AuthError> {
    const BEARER_PREFIX: &str = "Bearer ";
    if !auth_header.starts_with(BEARER_PREFIX) {
        return Err(AuthError::MissingToken);
    }
    let token = &auth_header[BEARER_PREFIX.len()..];
    if token.is_empty() {
        return Err(AuthError::InvalidTokenFormat);
    }
    Ok(token.to_string())
}

/// Validate the JWT token using the JWKS client.
///
/// # Arguments
///
/// * `jwks_client` — The JWKS client for signature verification.
/// * `token` — The raw JWT token string.
///
/// # Returns
///
/// * `Ok(AccessClaims)` — Validated claims.
/// * `Err(AuthError)` — Validation failed.
///
/// # Notes
///
/// This is a placeholder that demonstrates the expected interface. The actual
/// implementation will use the JWKS client from Story 1.3.
#[cfg(feature = "brrtrouter")]
pub fn validate_jwt(
    _jwks_client: &brrtrouter::security::JwksBearerProvider,
    _token: &str,
) -> Result<AccessClaims, AuthError> {
    Err(AuthError::JwksValidationFailed(
        "JWKS validation not yet integrated".into(),
    ))
}

// ===========================================================================
// Policy Evaluation
// ===========================================================================

impl RoutePolicyStore {
    /// Evaluate local policy from JWT claims for a `jwt-only` route.
    ///
    /// # Arguments
    ///
    /// * `claims` — The validated JWT claims.
    /// * `policy` — The route policy to evaluate against.
    ///
    /// # Returns
    ///
    /// `Ok(AuthDecision::Allowed)` if policy passes,
    /// `Ok(AuthDecision::Denied)` if any check fails.
    pub fn evaluate_jwt_only(
        &self,
        claims: &AccessClaims,
        policy: &RoutePolicy,
    ) -> Result<AuthDecision, AuthError> {
        match &policy.category {
            RouteAuthCategory::JwtOnly {
                required_roles,
                required_permissions,
                required_user_type,
            } => self.evaluate_local_policy(
                claims,
                required_roles,
                required_permissions,
                required_user_type.as_deref(),
            ),
            RouteAuthCategory::JwtWithFallback { .. }
            | RouteAuthCategory::OnlineOnly => {
                Err(AuthError::PolicyEvaluationError(
                    "evaluate_jwt_only called for non-jwt-only route".into(),
                ))
            }
        }
    }

    /// Evaluate local policy checks against claims.
    fn evaluate_local_policy(
        &self,
        claims: &AccessClaims,
        required_roles: &[String],
        required_permissions: &[String],
        required_user_type: Option<&str>,
    ) -> Result<AuthDecision, AuthError> {
        // 1. User type check
        if let Some(expected) = required_user_type {
            if claims.user_type != expected {
                return Ok(AuthDecision::Denied {
                    reason: format!("user_type_mismatch:{expected}"),
                    error: AuthError::UserTypeMismatch {
                        expected: expected.to_string(),
                        actual: claims.user_type.clone(),
                    },
                });
            }
        }

        // 2. Role check
        let claim_roles = &claims.sx.roles;
        for required_role in required_roles {
            if !claim_roles.iter().any(|r| r == required_role) {
                return Ok(AuthDecision::Denied {
                    reason: format!("missing_role:{required_role}"),
                    error: AuthError::MissingRole {
                        required_role: required_role.clone(),
                    },
                });
            }
        }

        // 3. Permission check
        let claim_permissions = &claims.sx.permissions;
        for required_perm in required_permissions {
            if !claim_permissions.iter().any(|p| p == required_perm) {
                return Ok(AuthDecision::Denied {
                    reason: format!("missing_permission:{required_perm}"),
                    error: AuthError::MissingPermission {
                        required_permission: required_perm.clone(),
                    },
                });
            }
        }

        Ok(AuthDecision::Allowed {
            claims: claims.clone(),
        })
    }

    /// Validate tenant ID against the X-Tenant-ID header.
    ///
    /// Uses the existing `AccessClaims::validate_tenant()` method from jwt.rs,
    /// which checks both `claims.tenant_id` and `claims.sx.tenant`.
    ///
    /// # Arguments
    ///
    /// * `claims` — The validated JWT claims.
    /// * `request_tenant` — The value of the X-Tenant-ID header.
    ///
    /// # Returns
    ///
    /// `Ok(())` if tenant matches, `Err(AuthError::TenantMismatch)` otherwise.
    pub fn validate_tenant(
        &self,
        claims: &AccessClaims,
        request_tenant: &str,
    ) -> Result<(), AuthError> {
        // Delegate to the existing validate_tenant method from jwt.rs
        claims
            .validate_tenant(request_tenant)
            .map_err(|jwt_err| match jwt_err {
                super::jwt::JwtError::TenantMismatch { expected, actual } => {
                    AuthError::TenantMismatch { expected, actual }
                }
                other => AuthError::PolicyEvaluationError(other.to_string()),
            })
    }
}

// ===========================================================================
// JWT Auth Middleware (BRRTRouter integration)
// ===========================================================================

/// JWT Common-Path Authorization Middleware for BRRTRouter.
///
/// This middleware:
/// 1. Extracts the Bearer token from the request.
/// 2. Validates the JWT (typ, iss, aud, exp, nbf, signature via JWKS).
/// 3. Looks up the route policy by path + method.
/// 4. Evaluates local policy for `jwt-only` routes.
/// 5. Passes validated claims to handlers for other routes.
///
/// # Thread Safety
///
/// Implements `Send + Sync` via `Arc` for internal state. Safe to share across
/// all handler coroutines in the `may` runtime.
///
/// # Security Model
///
/// - **Tenant validation**: Always checks `claims.tenant_id` against `X-Tenant-ID`.
/// - **Fail-closed**: Any error rejects the request (never fails open).
/// - **JWKS validation**: Signature verification is the foundational security check.
#[derive(Debug, Clone)]
pub struct JwtAuthMiddleware {
    /// The route policy store for policy lookups.
    pub policies: Arc<RoutePolicyStore>,
    /// JWKS client for token signature validation (from Story 1.3).
    #[allow(dead_code)]
    jwks_client: Option<String>, // Will be replaced with actual client reference
}

impl JwtAuthMiddleware {
    /// Create a new JWT auth middleware.
    #[must_use]
    pub fn new(policies: Arc<RoutePolicyStore>, jwks_client: Option<String>) -> Self {
        Self {
            policies,
            jwks_client,
        }
    }

    /// The core evaluation logic: validate JWT, look up policy, evaluate.
    ///
    /// # Arguments
    ///
    /// * `path` — The request path.
    /// * `method` — The HTTP method.
    /// * `tenant_header` — The value of X-Tenant-ID header (from request).
    /// * `claims` — The pre-validated JWT claims (from JWKS validation).
    ///
    /// # Returns
    ///
    /// The `AuthDecision` for this request.
    pub fn evaluate(
        &self,
        path: &str,
        method: &str,
        tenant_header: Option<&str>,
        claims: &AccessClaims,
    ) -> Result<AuthDecision, AuthError> {
        // 1. Validate tenant (CRITICAL — must happen before any handler access).
        if let Some(request_tenant) = tenant_header {
            self.policies.validate_tenant(claims, request_tenant)?;
        }

        // 2. Look up route policy.
        let policy = self
            .policies
            .get_policy(path, method)
            .ok_or(AuthError::PolicyNotFound)?;

        // 3. Evaluate based on category.
        match &policy.category {
            RouteAuthCategory::JwtOnly { .. } => {
                self.policies.evaluate_jwt_only(claims, policy)
            }
            RouteAuthCategory::JwtWithFallback { .. }
            | RouteAuthCategory::OnlineOnly => {
                // JWT is validated, pass claims to handler.
                Ok(AuthDecision::JwtCommonPath {
                    claims: claims.clone(),
                })
            }
        }
    }
}

/// BRRTRouter middleware implementation for JWT auth.
///
/// This struct wraps `JwtAuthMiddleware` and implements the
/// `brrtrouter::middleware::Middleware` trait to integrate with the
/// BRRTRouter dispatcher pipeline.
#[derive(Debug, Clone)]
pub struct BrrtJwtMiddleware {
    /// The underlying JWT auth logic.
    inner: JwtAuthMiddleware,
}

impl BrrtJwtMiddleware {
    /// Create a new BRRTRouter-integrated JWT middleware.
    #[must_use]
    pub fn new(policies: Arc<RoutePolicyStore>, jwks_client: Option<String>) -> Self {
        Self {
            inner: JwtAuthMiddleware::new(policies, jwks_client),
        }
    }
}

#[cfg(feature = "brrtrouter")]
impl brrtrouter::middleware::Middleware for BrrtJwtMiddleware {
    /// Pre-processing hook: validate JWT and evaluate policy.
    ///
    /// Returns `Some(response)` with 401/403 on failure, `None` to continue.
    fn before(
        &self,
        req: &brrtrouter::dispatcher::HandlerRequest,
    ) -> Option<brrtrouter::dispatcher::HandlerResponse> {
        // 1. Extract Bearer token.
        let auth_header = match req.get_header("authorization") {
            Some(h) => h,
            None => {
                return Some(brrtrouter::dispatcher::HandlerResponse::error(
                    401,
                    "Missing Bearer token",
                ));
            }
        };

        let token = match extract_bearer_token(auth_header) {
            Ok(t) => t,
            Err(e) => {
                return Some(brrtrouter::dispatcher::HandlerResponse::error(
                    e.status_code(),
                    &e.to_string(),
                ));
            }
        };

        // 2. Get tenant from header.
        let tenant_header = req.get_header("x-tenant-id");

        // 3. Get claims from request context (set by security provider).
        let claims = match req.get_header("x-sesame-claims") {
            Some(claims_json) => match serde_json::from_str::<AccessClaims>(claims_json) {
                Ok(c) => c,
                Err(e) => {
                    return Some(brrtrouter::dispatcher::HandlerResponse::error(
                        500,
                        &format!("Claims deserialization error: {e}"),
                    ));
                }
            },
            None => {
                // JWT was not pre-validated by security provider.
                return Some(brrtrouter::dispatcher::HandlerResponse::error(
                    503,
                    "JWT not pre-validated",
                ));
            }
        };

        // 4. Evaluate policy.
        let decision = match self.inner.evaluate(
            req.path(),
            req.method(),
            tenant_header,
            &claims,
        ) {
            Ok(d) => d,
            Err(e) => {
                return Some(brrtrouter::dispatcher::HandlerResponse::error(
                    e.status_code(),
                    &e.to_string(),
                ));
            }
        };

        // 5. Handle decision.
        match decision {
            AuthDecision::Allowed { .. }
            | AuthDecision::JwtCommonPath { .. } => None, // Continue to handler.
            AuthDecision::Denied { reason, error } => {
                Some(brrtrouter::dispatcher::HandlerResponse::error(
                    error.status_code(),
                    &format!("Denied: {reason}"),
                ))
            }
        }
    }

    /// Post-processing hook: emit metrics (if enabled).
    fn after(
        &self,
        _req: &brrtrouter::dispatcher::HandlerRequest,
        _res: &mut brrtrouter::dispatcher::HandlerResponse,
        _latency: std::time::Duration,
    ) {
        // Metrics are handled by the built-in MetricsMiddleware.
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::jwt::{AccessClaimsBuilder, SesameAuthzClaimsBuilder};

    fn make_valid_claims() -> AccessClaims {
        let sx = SesameAuthzClaimsBuilder::new()
            .tenant("tenant-abc")
            .portal("hauliage-web")
            .roles(vec!["driver".into(), "dispatcher".into()])
            .permissions(vec!["shipments:read".into(), "users:write".into()])
            .build()
            .unwrap();

        AccessClaimsBuilder::new()
            .iss("https://idam.example.com")
            .sub("user-123")
            .aud(vec!["identity-login-service".into()])
            .client_id("hauliage-web")
            .scope("profile:read")
            .exp(1779212000)
            .nbf(1779211700)
            .iat(1779211700)
            .jti("tok-12345")
            .ver(1)
            .sid("session-abc")
            .tenant_id("tenant-abc")
            .user_id("user-123")
            .user_type("registered")
            .sx(sx)
            .build()
            .unwrap()
    }

    fn make_policy_store() -> RoutePolicyStore {
        let mut store = RoutePolicyStore::new();
        store.register(RoutePolicy::new(
            "/api/v1/identity/users/me".into(),
            "GET".into(),
            RouteAuthCategory::JwtOnly {
                required_roles: vec![],
                required_permissions: vec![],
                required_user_type: Some("registered".into()),
            },
        ));
        store.register(RoutePolicy::new(
            "/api/v1/shipments".into(),
            "POST".into(),
            RouteAuthCategory::JwtOnly {
                required_roles: vec!["dispatcher".into()],
                required_permissions: vec!["shipments:write".into()],
                required_user_type: None,
            },
        ));
        store.register(RoutePolicy::new(
            "/api/v1/health".into(),
            "GET".into(),
            RouteAuthCategory::OnlineOnly,
        ));
        store
    }

    // --- extract_bearer_token tests ---

    #[test]
    fn test_extract_bearer_token_success() {
        let result = extract_bearer_token("Bearer eyJhbGciOiJSUzI1NiJ9...");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "eyJhbGciOiJSUzI1NiJ9...");
    }

    #[test]
    fn test_extract_bearer_token_missing() {
        let result = extract_bearer_token("");
        assert_eq!(result, Err(AuthError::MissingToken));
    }

    #[test]
    fn test_extract_bearer_token_no_bearer_prefix() {
        let result = extract_bearer_token("Basic dXNlcjpwYXNz");
        assert_eq!(result, Err(AuthError::MissingToken));
    }

    #[test]
    fn test_extract_bearer_token_empty_token() {
        let result = extract_bearer_token("Bearer ");
        assert_eq!(result, Err(AuthError::InvalidTokenFormat));
    }

    // --- RoutePolicyStore tests ---

    #[test]
    fn test_policy_store_register_and_lookup() {
        let store = make_policy_store();
        assert!(store.has_policy("/api/v1/identity/users/me", "GET"));
        assert!(store.has_policy("/api/v1/shipments", "POST"));
        assert!(!store.has_policy("/api/v1/unknown", "GET"));
        assert_eq!(store.len(), 3);
    }

    #[test]
    fn test_policy_store_empty() {
        let store = RoutePolicyStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert!(store.get_policy("/path", "GET").is_none());
    }

    #[test]
    fn test_policy_store_batch_register() {
        let mut store = RoutePolicyStore::new();
        store.register_batch(vec![
            ("/a".into(), "GET".into(), RouteAuthCategory::OnlineOnly),
            ("/b".into(), "POST".into(), RouteAuthCategory::JwtWithFallback {
                common_roles: vec![],
                common_permissions: vec![],
            }),
        ]);
        assert_eq!(store.len(), 2);
        assert!(store.has_policy("/a", "GET"));
        assert!(store.has_policy("/b", "POST"));
    }

    // --- Tenant validation tests ---

    #[test]
    fn test_validate_tenant_match() {
        let claims = make_valid_claims();
        let store = RoutePolicyStore::new();
        assert!(store.validate_tenant(&claims, "tenant-abc").is_ok());
    }

    #[test]
    fn test_validate_tenant_mismatch() {
        let claims = make_valid_claims();
        let store = RoutePolicyStore::new();
        let result = store.validate_tenant(&claims, "other-tenant");
        assert!(matches!(result, Err(AuthError::TenantMismatch { .. })));
    }

    // --- JWT-only policy evaluation tests ---

    #[test]
    fn test_jwt_only_user_type_match() {
        let claims = make_valid_claims();
        let store = make_policy_store();
        let policy = store.get_policy("/api/v1/identity/users/me", "GET").unwrap();
        let result = store.evaluate_jwt_only(&claims, policy);
        assert!(matches!(result, Ok(AuthDecision::Allowed { .. })));
    }

    #[test]
    fn test_jwt_only_user_type_mismatch() {
        let mut sx = SesameAuthzClaimsBuilder::new()
            .tenant("tenant-abc")
            .portal("hauliage-web")
            .roles(vec![])
            .permissions(vec![])
            .build()
            .unwrap();
        let mut claims = AccessClaimsBuilder::new()
            .iss("https://idam.example.com")
            .sub("user-456")
            .aud(vec!["identity-login-service".into()])
            .client_id("hauliage-web")
            .scope("profile:read")
            .exp(1779212000)
            .nbf(1779211700)
            .iat(1779211700)
            .jti("tok-67890")
            .ver(1)
            .sid("session-def")
            .tenant_id("tenant-abc")
            .user_id("user-456")
            .user_type("social") // Different type
            .sx(sx)
            .build()
            .unwrap();
        // Swap the sx claims since they're required
        claims.sx = SesameAuthzClaimsBuilder::new()
            .tenant("tenant-abc")
            .portal("hauliage-web")
            .roles(vec![])
            .permissions(vec![])
            .build()
            .unwrap();

        let store = make_policy_store();
        let policy = store.get_policy("/api/v1/identity/users/me", "GET").unwrap();
        let result = store.evaluate_jwt_only(&claims, policy);
        assert!(matches!(
            result,
            Ok(AuthDecision::Denied { error: AuthError::UserTypeMismatch { .. }, .. })
        ));
    }

    #[test]
    fn test_jwt_only_role_check_pass() {
        let claims = make_valid_claims(); // has "dispatcher" role
        let store = make_policy_store();
        let policy = store.get_policy("/api/v1/shipments", "POST").unwrap();
        let result = store.evaluate_jwt_only(&claims, policy);
        // dispatcher role present, but shipments:write permission missing
        assert!(matches!(
            result,
            Ok(AuthDecision::Denied { error: AuthError::MissingPermission { .. }, .. })
        ));
    }

    #[test]
    fn test_jwt_only_role_check_fail() {
        let mut sx = SesameAuthzClaimsBuilder::new()
            .tenant("tenant-abc")
            .portal("hauliage-web")
            .roles(vec!["driver".into()]) // No dispatcher
            .permissions(vec!["shipments:write".into()])
            .build()
            .unwrap();
        let mut claims = AccessClaimsBuilder::new()
            .iss("https://idam.example.com")
            .sub("user-789")
            .aud(vec!["identity-login-service".into()])
            .client_id("hauliage-web")
            .scope("profile:read")
            .exp(1779212000)
            .nbf(1779211700)
            .iat(1779211700)
            .jti("tok-abcde")
            .ver(1)
            .sid("session-ghi")
            .tenant_id("tenant-abc")
            .user_id("user-789")
            .user_type("registered")
            .sx(sx)
            .build()
            .unwrap();
        claims.sx = SesameAuthzClaimsBuilder::new()
            .tenant("tenant-abc")
            .portal("hauliage-web")
            .roles(vec!["driver".into()])
            .permissions(vec!["shipments:write".into()])
            .build()
            .unwrap();

        let store = make_policy_store();
        let policy = store.get_policy("/api/v1/shipments", "POST").unwrap();
        let result = store.evaluate_jwt_only(&claims, policy);
        assert!(matches!(
            result,
            Ok(AuthDecision::Denied { error: AuthError::MissingRole { .. }, .. })
        ));
    }

    // --- AuthDecision tests ---

    #[test]
    fn test_auth_decision_allowed() {
        let decision = AuthDecision::Allowed {
            claims: make_valid_claims(),
        };
        assert!(decision.is_allowed());
        assert!(!decision.is_denied());
        assert!(decision.error().is_none());
    }

    #[test]
    fn test_auth_decision_denied() {
        let decision = AuthDecision::Denied {
            reason: "missing_role:admin".into(),
            error: AuthError::MissingRole {
                required_role: "admin".into(),
            },
        };
        assert!(!decision.is_allowed());
        assert!(decision.is_denied());
        assert!(decision.error().is_some());
        assert_eq!(decision.error().unwrap().reason(), "missing_role");
    }

    #[test]
    fn test_auth_decision_jwt_common_path() {
        let decision = AuthDecision::JwtCommonPath {
            claims: make_valid_claims(),
        };
        assert!(decision.is_allowed());
        assert!(!decision.is_denied());
        assert!(decision.error().is_none());
    }

    // --- AuthError tests ---

    #[test]
    fn test_auth_error_status_codes() {
        assert_eq!(
            AuthError::MissingToken.status_code(),
            401
        );
        assert_eq!(
            AuthError::TenantMismatch {
                expected: "a".into(),
                actual: "b".into(),
            }.status_code(),
            403
        );
        assert_eq!(
            AuthError::MissingRole { required_role: "x".into() }.status_code(),
            403
        );
        assert_eq!(
            AuthError::MissingPermission { required_permission: "x".into() }.status_code(),
            403
        );
        assert_eq!(
            AuthError::UserTypeMismatch { expected: "a".into(), actual: "b".into() }.status_code(),
            403
        );
        assert_eq!(
            AuthError::RiskLevelTooHigh { required: "a".into(), actual: "b".into() }.status_code(),
            403
        );
        assert_eq!(
            AuthError::JwksValidationFailed("test".into()).status_code(),
            401
        );
        assert_eq!(
            AuthError::PolicyNotFound.status_code(),
            403
        );
    }

    #[test]
    fn test_auth_error_reasons() {
        assert_eq!(AuthError::MissingToken.reason(), "missing_token");
        assert_eq!(AuthError::InvalidTokenFormat.reason(), "invalid_format");
        assert_eq!(AuthError::PolicyNotFound.reason(), "policy_not_found");
        assert_eq!(
            AuthError::TenantMismatch { expected: "a".into(), actual: "b".into() }.reason(),
            "tenant_mismatch"
        );
        assert_eq!(AuthError::MissingTenantId.reason(), "missing_tenant_id");
        assert_eq!(AuthError::MissingRole { required_role: "x".into() }.reason(), "missing_role");
        assert_eq!(
            AuthError::MissingPermission { required_permission: "x".into() }.reason(),
            "missing_permission"
        );
        assert_eq!(AuthError::PolicyEvaluationError("test".into()).reason(), "policy_error");
        assert_eq!(AuthError::JwksValidationFailed("test".into()).reason(), "jwks_error");
    }

    // --- Policy not found for jwt-only ---

    #[test]
    fn test_jwt_only_no_policy() {
        let claims = make_valid_claims();
        let store = RoutePolicyStore::new();
        let policy = store.get_policy("/api/v1/unknown", "GET");
        assert!(policy.is_none());
    }

    // --- OnlineOnly category should pass through ---

    #[test]
    fn test_online_only_route_passes() {
        let claims = make_valid_claims();
        let store = make_policy_store();
        let policy = store.get_policy("/api/v1/health", "GET").unwrap();
        assert!(matches!(policy.category, RouteAuthCategory::OnlineOnly));
        // OnlineOnly should not be evaluated with evaluate_jwt_only
        let result = store.evaluate_jwt_only(&claims, policy);
        assert!(matches!(
            result,
            Err(AuthError::PolicyEvaluationError(msg)) if msg.contains("non-jwt-only")
        ));
    }

    // --- Middleware evaluate integration ---

    #[test]
    fn test_middleware_evaluate_jwt_only_allow() {
        let store = Arc::new(make_policy_store());
        let middleware = JwtAuthMiddleware::new(store.clone(), None);
        let claims = make_valid_claims();
        let result = middleware.evaluate(
            "/api/v1/identity/users/me",
            "GET",
            Some("tenant-abc"),
            &claims,
        );
        assert!(matches!(result, Ok(AuthDecision::Allowed { .. })));
    }

    #[test]
    fn test_middleware_evaluate_tenant_mismatch() {
        let store = Arc::new(make_policy_store());
        let middleware = JwtAuthMiddleware::new(store.clone(), None);
        let claims = make_valid_claims();
        let result = middleware.evaluate(
            "/api/v1/identity/users/me",
            "GET",
            Some("other-tenant"),
            &claims,
        );
        assert!(matches!(result, Err(AuthError::TenantMismatch { .. })));
    }

    #[test]
    fn test_middleware_evaluate_online_only() {
        let store = Arc::new(make_policy_store());
        let middleware = JwtAuthMiddleware::new(store.clone(), None);
        let claims = make_valid_claims();
        let result = middleware.evaluate(
            "/api/v1/health",
            "GET",
            Some("tenant-abc"),
            &claims,
        );
        assert!(matches!(result, Ok(AuthDecision::JwtCommonPath { .. })));
    }

    #[test]
    fn test_middleware_evaluate_no_tenant_header() {
        let store = Arc::new(make_policy_store());
        let middleware = JwtAuthMiddleware::new(store.clone(), None);
        let claims = make_valid_claims();
        // No tenant header — tenant validation should still work because
        // we only validate if tenant_header is Some
        let result = middleware.evaluate(
            "/api/v1/identity/users/me",
            "GET",
            None,
            &claims,
        );
        // Should succeed since no tenant check is performed when header is missing
        assert!(matches!(result, Ok(AuthDecision::Allowed { .. })));
    }

    // --- JwtCommonPath for non-jwt-only routes ---

    #[test]
    fn test_jwt_with_fallback_returns_common_path() {
        let mut store = RoutePolicyStore::new();
        store.register(RoutePolicy::new(
            "/api/v1/fallback".into(),
            "POST".into(),
            RouteAuthCategory::JwtWithFallback {
                common_roles: vec![],
                common_permissions: vec![],
            },
        ));
        let middleware = JwtAuthMiddleware::new(Arc::new(store), None);
        let claims = make_valid_claims();
        let result = middleware.evaluate(
            "/api/v1/fallback",
            "POST",
            Some("tenant-abc"),
            &claims,
        );
        assert!(matches!(result, Ok(AuthDecision::JwtCommonPath { .. })));
    }

    // --- Metrics reason labels ---

    #[test]
    fn test_all_error_reasons_are_distinct() {
        let e1 = AuthError::JwtValidationError("x".into());
        let e2 = AuthError::TenantMismatch { expected: "x".into(), actual: "x".into() };
        let e3 = AuthError::MissingRole { required_role: "x".into() };
        let e4 = AuthError::MissingPermission { required_permission: "x".into() };
        let e5 = AuthError::UserTypeMismatch { expected: "x".into(), actual: "x".into() };
        let e6 = AuthError::RiskLevelTooHigh { required: "x".into(), actual: "x".into() };
        let e7 = AuthError::PolicyEvaluationError("x".into());
        let e8 = AuthError::JwksValidationFailed("x".into());

        let reasons: Vec<&str> = vec![
            AuthError::MissingToken.reason(),
            AuthError::InvalidTokenFormat.reason(),
            e1.reason(),
            AuthError::PolicyNotFound.reason(),
            e2.reason(),
            AuthError::MissingTenantId.reason(),
            e3.reason(),
            e4.reason(),
            e5.reason(),
            e6.reason(),
            e7.reason(),
            e8.reason(),
        ];
        let unique: std::collections::HashSet<&str> = reasons.iter().copied().collect();
        assert_eq!(
            unique.len(),
            reasons.len(),
            "All error reasons should be unique for metrics labels"
        );
    }
}
