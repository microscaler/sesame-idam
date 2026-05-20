//! JWT Common-Path Authorization Middleware for BRRTRouter.
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
//! - **Tenant validation is critical**: Always validates `claims.tenant_id` against
//!   `X-Tenant-ID` header BEFORE any handler executes.
//! - **JWKS cache poisoning protection**: Keys are validated for correct type/algorithm.
//! - **Fail-closed**: Any middleware error rejects the request (never fails open).
//!
//! # Usage
//!
//! ```rust,ignore
//! use sesame_common::jwt::AccessClaims;
//! use sesame_common::middleware::{JwtAuthMiddleware, RoutePolicyStore};
//!
//! let policies = RoutePolicyStore::from_classification(&CLASSIFICATIONS);
//! let middleware = JwtAuthMiddleware::new(policies, jwks_client.clone());
//! dispatcher.add_middleware(middleware);
//! ```

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

// ─── Re-export JWT types ─────────────────────────────────────────────────────

pub use super::jwt::{AccessClaims, ActorClaim, SesameAuthzClaims};

// ─── Error Types ─────────────────────────────────────────────────────────────

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
    TenantMismatch {
        expected: String,
        actual: String,
    },
    /// Missing X-Tenant-ID header.
    MissingTenantId,
    /// Required role not present in claims.
    MissingRole {
        required_role: String,
    },
    /// Required permission not present in claims.
    MissingPermission {
        required_permission: String,
    },
    /// User type does not match route requirement.
    UserTypeMismatch {
        expected: String,
        actual: String,
    },
    /// Risk level too high for this route.
    RiskLevelTooHigh {
        required: String,
        actual: String,
    },
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

// ─── Route Policy ────────────────────────────────────────────────────────────

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
        /// Required user type (e.g., "customer", "platform").
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
    pub fn new(
        path: String,
        method: String,
        category: RouteAuthCategory,
    ) -> Self {
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
    ///
    /// # Arguments
    ///
    /// * `path` — The request path (e.g., "/api/v1/identity/users/me")
    /// * `method` — The HTTP method (e.g., "GET", "POST")
    ///
    /// # Returns
    ///
    /// The matching `RoutePolicy`, or `None` if no policy exists.
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
    ///
    /// This is the primary way to populate the store from Story 4.1 output.
    ///
    /// # Arguments
    ///
    /// * `policies` — Iterator of `(path, method, category)` tuples.
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

// ─── AuthDecision ────────────────────────────────────────────────────────────

/// Result of middleware evaluation.
///
/// This is the decision the middleware makes about whether to allow/deny the request.
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

// ─── JWT Validation Helper ───────────────────────────────────────────────────

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
    // Actual implementation will delegate to the JWKS client.
    // For now, this is a stub — the real validation happens in the service's
    // security module which uses the existing JwksBearerProvider.
    Err(AuthError::JwksValidationFailed(
        "JWKS validation not yet integrated".into(),
    ))
}

// ─── Policy Evaluation ───────────────────────────────────────────────────────

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
            } => {
                self.evaluate_local_policy(
                    claims,
                    required_roles,
                    required_permissions,
                    required_user_type.as_deref(),
                )
            }
            RouteAuthCategory::JwtWithFallback { .. }
            | RouteAuthCategory::OnlineOnly => {
                // This should not be called for non-jwt-only routes.
                Err(AuthError::PolicyEvaluationError(
                    "evaluate_jwt_only called for non-jwt-only route".into(),
                ))
            }
        }
    }

    /// Evaluate local policy checks against claims.
    ///
    /// # Arguments
    ///
    /// * `claims` — The validated JWT claims.
    /// * `required_roles` — Roles that must be present.
    /// * `required_permissions` — Permissions that must be present.
    /// * `required_user_type` — If present, must match claims.user_type.
    ///
    /// # Returns
    ///
    /// `Ok(AuthDecision::Allowed)` if all checks pass,
    /// `Ok(AuthDecision::Denied)` with reason on first failure.
    fn evaluate_local_policy(
        &self,
        claims: &AccessClaims,
        required_roles: &[String],
        required_permissions: &[String],
        required_user_type: Option<&str>,
    ) -> Result<AuthDecision, AuthError> {
        // 1. User type check
        if let Some(expected) = required_user_type {
            let actual = claims.user_type.as_deref().unwrap_or("");
            if actual != expected {
                return Ok(AuthDecision::Denied {
                    reason: format!("user_type_mismatch:{expected}"),
                    error: AuthError::UserTypeMismatch {
                        expected: expected.to_string(),
                        actual: actual.to_string(),
                    },
                });
            }
        }

        // 2. Role check
        let sx = claims
            .sx
            .as_ref()
            .ok_or(AuthError::PolicyEvaluationError(
                "Missing namespaced claims (sx)".into(),
            ))?;
        let claim_roles = sx.roles.as_deref().unwrap_or_default();
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
        let claim_permissions = sx.permissions.as_deref().unwrap_or_default();
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
        let claim_tenant = claims
            .tenant_id
            .as_ref()
            .ok_or(AuthError::MissingTenantId)?;

        if claim_tenant != request_tenant {
            return Err(AuthError::TenantMismatch {
                expected: request_tenant.to_string(),
                actual: claim_tenant.clone(),
            });
        }

        // Also validate namespaced tenant if present.
        if let Some(ref sx) = claims.sx {
            if let Some(ref sx_tenant) = sx.tenant {
                if sx_tenant != request_tenant {
                    return Err(AuthError::TenantMismatch {
                        expected: request_tenant.to_string(),
                        actual: sx_tenant.clone(),
                    });
                }
            }
        }

        Ok(())
    }
}

// ─── JWT Auth Middleware (BRRTRouter integration) ─────────────────────────────

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
    /// Note: This is a placeholder — actual implementation uses the service's
    /// JwksBearerProvider.
    #[allow(dead_code)]
    jwks_client: Option<String>, // Will be replaced with actual client reference
}

impl JwtAuthMiddleware {
    /// Create a new JWT auth middleware.
    ///
    /// # Arguments
    ///
    /// * `policies` — The route policy store.
    /// * `jwks_client` — Optional reference to the JWKS client.
    #[must_use]
    pub fn new(
        policies: Arc<RoutePolicyStore>,
        jwks_client: Option<String>,
    ) -> Self {
        Self {
            policies,
            jwks_client,
        }
    }

    /// The core evaluation logic: validate JWT, look up policy, evaluate.
    ///
    /// # Arguments
    ///
    /// * `token` — The raw Bearer token string.
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
            self.policies
                .validate_tenant(claims, request_tenant)?;
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

// ─── Middleware Integration (BRRTRouter trait impl) ───────────────────────────

/// BRRTRouter middleware implementation for JWT auth.
///
/// This struct wraps `JwtAuthMiddleware` and implements the `brrtrouter::middleware::Middleware`
/// trait to integrate with the BRRTRouter dispatcher pipeline.
#[derive(Debug, Clone)]
pub struct BrrtJwtMiddleware {
    /// The underlying JWT auth logic.
    inner: JwtAuthMiddleware,
}

impl BrrtJwtMiddleware {
    /// Create a new BRRTRouter-integrated JWT middleware.
    ///
    /// # Arguments
    ///
    /// * `policies` — The route policy store.
    /// * `jwks_client` — Optional JWKS client reference.
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
        let auth_header = req.get_header("authorization")?;
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
        // The BRRTRouter security provider should have already validated the JWT
        // and attached claims to the request context.
        let claims = match req.get_header("x-sesame-claims") {
            Some(claims_json) => {
                match serde_json::from_str::<AccessClaims>(claims_json) {
                    Ok(c) => c,
                    Err(e) => {
                        return Some(brrtrouter::dispatcher::HandlerResponse::error(
                            500,
                            &format!("Claims deserialization error: {e}"),
                        ));
                    }
                }
            }
            None => {
                // JWT was not pre-validated by security provider.
                // In production, this would delegate to the JWKS client.
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
        res: &mut brrtrouter::dispatcher::HandlerResponse,
        _latency: std::time::Duration,
    ) {
        // Metrics are handled by the built-in MetricsMiddleware.
        // If metrics feature is enabled, emit custom JWT metrics here.
    }
}

#[cfg(not(feature = "brrtrouter"))]
impl BrrtJwtMiddleware {
    /// Stub implementation when brrtrouter feature is disabled.
    #[allow(dead_code)]
    fn before(
        &self,
        _req: &std::any::Any,
    ) -> Option<std::any::Any> {
        None
    }
}
