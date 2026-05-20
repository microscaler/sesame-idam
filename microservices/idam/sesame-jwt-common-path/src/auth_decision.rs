//! # Auth Decision Types
//!
//! Defines the results of JWT validation and authorization evaluation.
//!
//! `AuthDecision` represents the outcome of the middleware's evaluation:
//! - `Allowed`: jwt-only route, local policy approved
//! - `Denied`: jwt-only route, local policy rejected
//! - `JwtCommonPath`: jwt-with-fallback or online-only, continue to handler
//!
//! `AuthError` represents the various failure modes with specific error types
//! for security auditing and accurate HTTP status mapping.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use sesame_common::AccessClaims;

// ---------------------------------------------------------------------------
// AuthDecision — result of middleware evaluation
// ---------------------------------------------------------------------------

/// The result of JWT validation and authorization evaluation.
///
/// This enum represents the three possible outcomes:
///
/// 1. **Allowed** — jwt-only route, local policy approved. The request proceeds
///    to the handler with `AccessClaims` in context.
/// 2. **Denied** — jwt-only route, local policy rejected. The client receives
///    a 403 Forbidden with the denial reason.
/// 3. **JwtCommonPath** — jwt-with-fallback or online-only route. The JWT has
///    been validated but the authorization decision is deferred to the handler
///    (which may call authz-core for online evaluation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthDecision {
    /// JWT-only route approved. Includes the validated claims for handler context.
    Allowed { claims: AccessClaims },

    /// JWT-only route denied. Includes the reason for denial.
    /// Clients receive 403 Forbidden.
    Denied { reason: String },

    /// JWT validated, policy evaluation deferred to handler.
    /// Used for jwt-with-fallback and online-only routes.
    JwtCommonPath { claims: AccessClaims },
}

impl AuthDecision {
    /// Returns true if the decision is to allow the request through.
    #[must_use]
    pub fn is_allowed(&self) -> bool {
        matches!(self, AuthDecision::Allowed { .. })
    }

    /// Returns true if the decision is to deny the request.
    #[must_use]
    pub fn is_denied(&self) -> bool {
        matches!(self, AuthDecision::Denied { .. })
    }

    /// Returns true if the decision is to continue to the handler.
    #[must_use]
    pub fn is_continued(&self) -> bool {
        matches!(self, AuthDecision::JwtCommonPath { .. })
    }

    /// Returns the reason for denial, if denied.
    #[must_use]
    pub fn denial_reason(&self) -> Option<&str> {
        match self {
            AuthDecision::Denied { reason } => Some(reason),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// AuthError — types of authorization failures
// ---------------------------------------------------------------------------

/// Errors produced during JWT validation and authorization.
///
/// Each variant maps to a specific HTTP status code:
/// - `MissingAuthorization` → 401 Unauthorized
/// - `InvalidBearerScheme` → 401 Unauthorized
/// - `MissingJwt` → 401 Unauthorized
/// - `JwtInvalid` → 401 Unauthorized
/// - `JwtExpired` → 401 Unauthorized
/// - `MissingTenantId` → 401 Unauthorized
/// - `TenantMismatch` → 401 Unauthorized
/// - `RoleCheckFailed` → 403 Forbidden
/// - `PermissionCheckFailed` → 403 Forbidden
/// - `RiskCheckFailed` → 403 Forbidden
/// - `PolicyNotFound` → 500 Internal Server Error
/// - `InternalError` → 503 Service Unavailable
///
/// # Security
///
/// - Error responses MUST NOT leak internal details. Use generic messages for
///   external responses and detailed logging for internal monitoring.
/// - All failures must reject (never fail open) per HACK-405.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum AuthError {
    /// Missing Authorization header.
    #[error("missing Authorization header")]
    MissingAuthorization,

    /// Authorization header does not use the Bearer scheme.
    #[error("invalid Authorization scheme: expected Bearer")]
    InvalidBearerScheme,

    /// No JWT token present in the Bearer token.
    #[error("no token found in Authorization header")]
    MissingJwt,

    /// JWT token is malformed (not valid base64url segments).
    #[error("malformed JWT: {0}")]
    JwtInvalid(String),

    /// JWT token has expired.
    #[error("JWT expired at {exp}")]
    JwtExpired { exp: i64 },

    /// JWT token is not yet valid (nbf in the future).
    #[error("JWT not yet valid until {nbf}")]
    JwtNotYetValid { nbf: i64 },

    /// JWT signature verification failed.
    #[error("JWT signature verification failed")]
    JwtSignatureInvalid,

    /// JWT typ claim is not `at+jwt`.
    #[error("JWT typ is not `at+jwt`: {typ}")]
    JwtWrongType { typ: String },

    /// JWT issuer does not match the expected issuer.
    #[error("JWT issuer mismatch: expected {expected}, got {actual}")]
    JwtIssuerMismatch { expected: String, actual: String },

    /// JWT audience does not include the expected audience.
    #[error("JWT audience mismatch: expected {expected}, got {actual}")]
    JwtAudienceMismatch { expected: String, actual: String },

    /// Route policy not found for the given path+method.
    /// This indicates a configuration error — the route should be classified.
    #[error("route policy not found: {path} {method}")]
    PolicyNotFound { path: String, method: String },

    /// Missing X-Tenant-ID header.
    /// All routes MUST have this header present.
    #[error("missing X-Tenant-ID header")]
    MissingTenantId,

    /// X-Tenant-ID header does not match the tenant in JWT claims.
    /// This is a critical tenant isolation failure (HACK-401).
    #[error("tenant mismatch: expected {expected}, got {actual}")]
    TenantMismatch { expected: String, actual: String },

    /// JWT claims role check failed.
    #[error("role check failed: required roles {required:?}, claim has {actual:?}")]
    RoleCheckFailed {
        required: Vec<String>,
        actual: Vec<String>,
    },

    /// JWT claims permission check failed.
    #[error("permission check failed: required permission {required:?}, claim has {actual:?}")]
    PermissionCheckFailed {
        required: Vec<String>,
        actual: Vec<String>,
    },

    /// JWT claims risk check failed.
    #[error("risk check failed: route requires {required}, claim has {actual:?}")]
    RiskCheckFailed {
        required: String,
        actual: Option<String>,
    },

    /// Internal error — unexpected failure during middleware operation.
    /// All internal errors result in 503 (never fail open).
    #[error("internal error: {0}")]
    InternalError(String),
}

impl AuthError {
    /// Returns the HTTP status code for this error.
    ///
    /// Per HACK-405: all errors reject the request.
    #[must_use]
    pub fn http_status(&self) -> u16 {
        match self {
            Self::MissingAuthorization => 401,
            Self::InvalidBearerScheme => 401,
            Self::MissingJwt => 401,
            Self::JwtInvalid(_) => 401,
            Self::JwtExpired { .. } => 401,
            Self::JwtNotYetValid { .. } => 401,
            Self::JwtSignatureInvalid => 401,
            Self::JwtWrongType { .. } => 401,
            Self::JwtIssuerMismatch { .. } => 401,
            Self::JwtAudienceMismatch { .. } => 401,
            Self::PolicyNotFound { .. } => 500,
            Self::MissingTenantId => 401,
            Self::TenantMismatch { .. } => 401,
            Self::RoleCheckFailed { .. } => 403,
            Self::PermissionCheckFailed { .. } => 403,
            Self::RiskCheckFailed { .. } => 403,
            Self::InternalError(_) => 503,
        }
    }

    /// Returns a human-readable reason suitable for external error responses.
    ///
    /// Internal details are stripped from error messages to prevent information leakage.
    #[must_use]
    pub fn external_reason(&self) -> String {
        match self {
            Self::MissingAuthorization => "Authorization header is required".to_string(),
            Self::InvalidBearerScheme => "Only Bearer tokens are accepted".to_string(),
            Self::MissingJwt => "No token provided".to_string(),
            Self::JwtInvalid(_) => "Invalid token format".to_string(),
            Self::JwtExpired { .. } => "Token has expired".to_string(),
            Self::JwtNotYetValid { .. } => "Token is not yet valid".to_string(),
            Self::JwtSignatureInvalid => "Token signature is invalid".to_string(),
            Self::JwtWrongType { .. } => "Token type is invalid".to_string(),
            Self::JwtIssuerMismatch { .. } => "Token issuer is not trusted".to_string(),
            Self::JwtAudienceMismatch { .. } => "Token audience is invalid".to_string(),
            Self::PolicyNotFound { .. } => "Service configuration error".to_string(),
            Self::MissingTenantId => "Tenant context is required".to_string(),
            Self::TenantMismatch { .. } => "Tenant mismatch".to_string(),
            Self::RoleCheckFailed { .. } => "Insufficient permissions".to_string(),
            Self::PermissionCheckFailed { .. } => "Insufficient permissions".to_string(),
            Self::RiskCheckFailed { .. } => "Insufficient permissions".to_string(),
            Self::InternalError(_) => "Service unavailable".to_string(),
        }
    }

    /// Returns true if this error should be logged as a security event.
    #[must_use]
    pub fn is_security_event(&self) -> bool {
        matches!(
            self,
            Self::TenantMismatch { .. }
                | Self::RoleCheckFailed { .. }
                | Self::PermissionCheckFailed { .. }
                | Self::RiskCheckFailed { .. }
                | Self::JwtSignatureInvalid
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_decision_allowed_is_allowed() {
        let decision = AuthDecision::Allowed {
            claims: AccessClaims::builder()
                .iss("https://idam.example.com")
                .sub("user-1")
                .aud(vec!["identity-login-service".into()])
                .client_id("test")
                .scope("read".into())
                .exp(9999999999)
                .nbf(0)
                .iat(0)
                .jti("jti-1")
                .ver(1)
                .sid("sid-1")
                .tenant_id("tenant-a")
                .user_id("user-1")
                .user_type("registered")
                .sx(sesame_common::SesameAuthzClaims::builder()
                    .tenant("tenant-a")
                    .portal("test")
                    .build()
                    .unwrap())
                .build()
                .unwrap(),
        };
        assert!(decision.is_allowed());
        assert!(!decision.is_denied());
        assert!(!decision.is_continued());
    }

    #[test]
    fn auth_decision_denied() {
        let decision = AuthDecision::Denied {
            reason: "jwt_only_policy_violation".into(),
        };
        assert!(!decision.is_allowed());
        assert!(decision.is_denied());
        assert!(!decision.is_continued());
        assert_eq!(decision.denial_reason(), Some("jwt_only_policy_violation"));
    }

    #[test]
    fn auth_decision_continued() {
        let decision = AuthDecision::JwtCommonPath {
            claims: AccessClaims::builder()
                .iss("https://idam.example.com")
                .sub("user-1")
                .aud(vec!["identity-login-service".into()])
                .client_id("test")
                .scope("read".into())
                .exp(9999999999)
                .nbf(0)
                .iat(0)
                .jti("jti-1")
                .ver(1)
                .sid("sid-1")
                .tenant_id("tenant-a")
                .user_id("user-1")
                .user_type("registered")
                .sx(sesame_common::SesameAuthzClaims::builder()
                    .tenant("tenant-a")
                    .portal("test")
                    .build()
                    .unwrap())
                .build()
                .unwrap(),
        };
        assert!(!decision.is_allowed());
        assert!(!decision.is_denied());
        assert!(decision.is_continued());
    }

    #[test]
    fn auth_error_http_status_mapping() {
        assert_eq!(AuthError::MissingAuthorization.http_status(), 401);
        assert_eq!(AuthError::InvalidBearerScheme.http_status(), 401);
        assert_eq!(AuthError::MissingJwt.http_status(), 401);
        assert_eq!(AuthError::JwtInvalid("test".into()).http_status(), 401);
        assert_eq!(AuthError::JwtExpired { exp: 123 }.http_status(), 401);
        assert_eq!(AuthError::MissingTenantId.http_status(), 401);
        assert_eq!(
            AuthError::TenantMismatch {
                expected: "a".into(),
                actual: "b".into(),
            }
            .http_status(),
            401
        );
        assert_eq!(
            AuthError::RoleCheckFailed {
                required: vec!["admin".into()],
                actual: vec!["user".into()],
            }
            .http_status(),
            403
        );
        assert_eq!(
            AuthError::PermissionCheckFailed {
                required: vec!["write".into()],
                actual: vec!["read".into()],
            }
            .http_status(),
            403
        );
        assert_eq!(
            AuthError::PolicyNotFound {
                path: "/test".into(),
                method: "GET".into(),
            }
            .http_status(),
            500
        );
        assert_eq!(AuthError::InternalError("test".into()).http_status(), 503);
    }

    #[test]
    fn auth_error_external_reason_sanitization() {
        // Ensure no internal details leak to external responses
        let error = AuthError::JwtInvalid("segment-parse-error".into());
        assert!(!error.external_reason().contains("segment-parse-error"));
        assert_eq!(error.external_reason(), "Invalid token format");

        let error = AuthError::InternalError("stack-trace-here".into());
        assert!(!error.external_reason().contains("stack-trace-here"));
        assert_eq!(error.external_reason(), "Service unavailable");
    }

    #[test]
    fn auth_error_security_events() {
        assert!(AuthError::TenantMismatch {
            expected: "a".into(),
            actual: "b".into(),
        }
        .is_security_event());
        assert!(AuthError::RoleCheckFailed {
            required: vec!["admin".into()],
            actual: vec!["user".into()],
        }
        .is_security_event());
        assert!(AuthError::JwtSignatureInvalid.is_security_event());
        assert!(!AuthError::MissingAuthorization.is_security_event());
        assert!(!AuthError::InternalError("test".into()).is_security_event());
    }
}
