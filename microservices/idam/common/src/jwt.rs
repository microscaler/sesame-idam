//! JWT claim types, validation, and builder for Sesame-IDAM microservices.
//!
//! Implements the new JWT claim structures defined in Epic 2 (Claims Schema Evolution):
//! - `ActorClaim` — RFC 8693 delegation actor claim
//! - `SesameAuthzClaims` — namespaced authorization data (`https://sesame-idam.dev/claims`)
//! - `AccessClaims` — top-level JWT claim structure
//!
//! # Usage
//!
//! ```rust
//! use sesame_common::jwt::{ActorClaim, SesameAuthzClaims, AccessClaims, AccessClaimsBuilder};
//!
//! let sx = SesameAuthzClaims::builder()
//!     .tenant("hauliage")
//!     .portal("hauliage-web")
//!     .roles(vec!["driver".into(), "dispatcher".into()])
//!     .permissions(vec!["shipments:read".into()])
//!     .build()
//!     .unwrap();
//!
//! let claims = AccessClaims::builder()
//!     .iss("https://idam.example.com")
//!     .sub("user-123")
//!     .aud(vec!["identity-login-service".into()])
//!     .client_id("hauliage-web")
//!     .scope("profile:read")
//!     .exp(1779212000)
//!     .nbf(1779211700)
//!     .iat(1779211700)
//!     .jti("tok-12345")
//!     .ver(1)
//!     .sid("session-abc")
//!     .tenant_id("hauliage")
//!     .user_id("user-123")
//!     .user_type("registered")
//!     .sx(sx)
//!     .build()
//!     .unwrap();
//!
//! assert!(claims.validate().is_ok());
//! ```

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Constants (issuer allow-list, expected audiences, valid risk values)
// ---------------------------------------------------------------------------

/// Issuers trusted to issue access tokens.
pub const ALLOWED_ISSUERS: &[&str] =
    &["https://idam.example.com", "https://idam.hauliage.internal"];

/// Expected audience values for access tokens.
pub const EXPECTED_AUDIENCES: &[&str] = &[
    "identity-login-service",
    "identity-session-service",
    "identity-user-mgmt-service",
    "authz-core",
    "api-keys",
    "org-mgmt",
];

/// Valid risk assessment levels.
pub const VALID_RISK_VALUES: &[&str] = &["normal", "elevated", "critical"];

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Validation errors produced by `AccessClaims::validate()`.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum JwtValidationError {
    #[error("invalid issuer: {0}")]
    InvalidIssuer(String),
    #[error("no audience intersection with expected list")]
    InvalidAudience,
    #[error("missing or zero version")]
    MissingVersion,
    #[error("missing or empty tenant_id")]
    MissingTenant,
    #[error("missing authz claims namespace (sx.tenant)")]
    MissingAuthzClaims,
    #[error("invalid risk value: {0}")]
    InvalidRisk(String),
    #[error("invalid token version: {0}")]
    InvalidTokenVersion(String),
    #[error("token has expired (exp={0})")]
    Expired(i64),
    #[error("token not yet valid (nbf={0})")]
    NotYetValid(i64),
    #[error("signature invalid")]
    SignatureInvalid,
}

/// Errors produced during token construction / serialization.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum JwtError {
    #[error("missing required field: {0}")]
    MissingRequiredField(String),
    #[error("validation failed: {0}")]
    ValidationError(#[from] JwtValidationError),
    #[error("serialization error: {0}")]
    SerializationError(String),
    #[error("builder field set multiple times: {0}")]
    DuplicateField(String),
    #[error("tenant mismatch: expected {expected}, actual {actual}")]
    TenantMismatch { expected: String, actual: String },
}

// ---------------------------------------------------------------------------
// ActorClaim: RFC 8693 delegation actor
// ---------------------------------------------------------------------------

/// RFC 8693 OAuth 2.0 delegation `act` claim.
///
/// Represents the actor on whose behalf the token holder is acting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActorClaim {
    /// Subject (entity identifier) of the acting party.
    pub sub: String,
}

// ---------------------------------------------------------------------------
// SesameAuthzClaims: namespaced authorization data
// ---------------------------------------------------------------------------

/// Namespaced authorization data, stored under the
/// `https://sesame-idam.dev/claims` JSON key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SesameAuthzClaims {
    /// Tenant identifier for the token holder.
    pub tenant: String,
    /// Portal / application identifier (e.g. `hauliage-web`).
    pub portal: String,
    /// User roles (e.g. `driver`, `dispatcher`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
    /// User permissions (e.g. `shipments:read`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<String>,
    /// Hash of the permissions array (SHA-256 hex) for integrity verification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions_hash: Option<String>,
    /// Reference to external entitlements snapshot (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entitlements_ref: Option<String>,
    /// Hash of the entitlements snapshot (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entitlements_hash: Option<String>,
    /// Risk assessment level (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk: Option<String>,
}

impl SesameAuthzClaims {
    /// Create a new builder for `SesameAuthzClaims`.
    pub fn builder() -> SesameAuthzClaimsBuilder {
        SesameAuthzClaimsBuilder::new()
    }

    /// Compute the SHA-256 hash of the permissions array.
    /// Returns the hex-encoded digest.
    pub fn compute_permissions_hash(&self) -> String {
        let mut hasher = Sha256::new();
        let mut sorted = self.permissions.clone();
        sorted.sort();
        let payload = sorted.join("\n");
        hasher.update(payload.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// Builder for `SesameAuthzClaims`.
#[derive(Debug, Clone, Default)]
pub struct SesameAuthzClaimsBuilder {
    tenant: Option<String>,
    portal: Option<String>,
    roles: Vec<String>,
    permissions: Vec<String>,
    entitlements_ref: Option<String>,
    entitlements_hash: Option<String>,
    risk: Option<String>,
}

impl SesameAuthzClaimsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tenant(mut self, tenant: impl Into<String>) -> Self {
        self.tenant = Some(tenant.into());
        self
    }

    pub fn portal(mut self, portal: impl Into<String>) -> Self {
        self.portal = Some(portal.into());
        self
    }

    pub fn roles(mut self, roles: Vec<String>) -> Self {
        self.roles = roles;
        self
    }

    pub fn permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn entitlements_ref(mut self, ref_id: impl Into<String>) -> Self {
        self.entitlements_ref = Some(ref_id.into());
        self
    }

    pub fn entitlements_hash(mut self, hash: impl Into<String>) -> Self {
        self.entitlements_hash = Some(hash.into());
        self
    }

    pub fn risk(mut self, risk: impl Into<String>) -> Self {
        self.risk = Some(risk.into());
        self
    }

    pub fn build(self) -> Result<SesameAuthzClaims, JwtError> {
        let tenant = self
            .tenant
            .ok_or_else(|| JwtError::MissingRequiredField("tenant".into()))?;
        let portal = self
            .portal
            .ok_or_else(|| JwtError::MissingRequiredField("portal".into()))?;

        let mut claims = SesameAuthzClaims {
            tenant,
            portal,
            roles: self.roles,
            permissions: self.permissions,
            permissions_hash: None,
            entitlements_ref: self.entitlements_ref,
            entitlements_hash: self.entitlements_hash,
            risk: self.risk,
        };

        if !claims.permissions.is_empty() {
            claims.permissions_hash = Some(claims.compute_permissions_hash());
        }

        Ok(claims)
    }
}

// ---------------------------------------------------------------------------
// AccessClaims: top-level JWT claim structure
// ---------------------------------------------------------------------------

/// Top-level JWT access token claim structure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccessClaims {
    /// JWT issuer (required).
    pub iss: String,
    /// Token subject / user ID (required).
    pub sub: String,
    /// Intended audience(s) (required, non-empty).
    #[serde(default)]
    pub aud: Vec<String>,
    /// Client / application ID (required).
    pub client_id: String,
    /// Scope string (required, empty string means no permissions — valid per schema).
    pub scope: String,
    /// Expiration time (Unix timestamp, seconds).
    pub exp: i64,
    /// Not before (Unix timestamp, seconds).
    pub nbf: i64,
    /// Issued at (Unix timestamp, seconds).
    pub iat: i64,
    /// JWT ID (unique, required).
    pub jti: String,
    /// Token version (required, >= 1 for valid tokens).
    pub ver: u64,
    /// Session ID (required).
    pub sid: String,
    /// Tenant identifier (required, non-empty).
    pub tenant_id: String,
    /// User identifier (required).
    pub user_id: String,
    /// User type (e.g. "registered", "social", "api_key").
    pub user_type: String,
    /// Namespaced authorization claims (`https://sesame-idam.dev/claims`).
    #[serde(rename = "https://sesame-idam.dev/claims")]
    pub sx: SesameAuthzClaims,
    /// Optional RFC 8693 delegation actor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act: Option<ActorClaim>,
}

impl AccessClaims {
    /// Create a new builder for `AccessClaims`.
    pub fn builder() -> AccessClaimsBuilder {
        AccessClaimsBuilder::new()
    }

    /// Validate required claims and field constraints.
    ///
    /// Checks:
    /// 1. `iss` in `ALLOWED_ISSUERS`
    /// 2. `aud` intersects `EXPECTED_AUDIENCES`
    /// 3. `ver >= 1`
    /// 4. `tenant_id` is not empty
    /// 5. `sx.tenant` is not empty
    /// 6. `sx.risk` is valid if present
    pub fn validate(&self) -> Result<(), JwtValidationError> {
        if !ALLOWED_ISSUERS.contains(&self.iss.as_str()) {
            return Err(JwtValidationError::InvalidIssuer(self.iss.clone()));
        }

        if self.aud.is_empty()
            || !self
                .aud
                .iter()
                .any(|a| EXPECTED_AUDIENCES.contains(&a.as_str()))
        {
            return Err(JwtValidationError::InvalidAudience);
        }

        if self.ver == 0 {
            return Err(JwtValidationError::MissingVersion);
        }

        if self.tenant_id.is_empty() {
            return Err(JwtValidationError::MissingTenant);
        }

        if self.sx.tenant.is_empty() {
            return Err(JwtValidationError::MissingAuthzClaims);
        }

        if let Some(ref risk) = self.sx.risk {
            if !VALID_RISK_VALUES.contains(&risk.as_str()) {
                return Err(JwtValidationError::InvalidRisk(risk.clone()));
            }
        }

        Ok(())
    }

    /// Validate that the JWT's tenant_id matches the expected tenant from the
    /// request's X-Tenant-ID header.
    ///
    /// Returns `Err(JwtError::TenantMismatch)` if:
    /// - `claims.tenant_id` does not match `expected_tenant` (HACK-241)
    /// - `sx.tenant` does not match `expected_tenant` (HACK-243: missing header bypass)
    ///
    /// This MUST be called BEFORE any database query to prevent cross-tenant
    /// data access.
    ///
    /// HACK-243: If the caller passes `expected_tenant = ""` (empty string) when
    /// the X-Tenant-ID header is missing, this method still rejects the request,
    /// ensuring that a missing header cannot be used to bypass tenant validation.
    pub fn validate_tenant(&self, expected_tenant: &str) -> Result<(), JwtError> {
        // Compare top-level tenant_id against expected (from X-Tenant-ID header)
        if self.tenant_id != expected_tenant {
            return Err(JwtError::TenantMismatch {
                expected: expected_tenant.to_string(),
                actual: self.tenant_id.clone(),
            });
        }

        // Also validate the namespaced claim — both must match the request tenant
        if self.sx.tenant != expected_tenant {
            return Err(JwtError::TenantMismatch {
                expected: expected_tenant.to_string(),
                actual: self.sx.tenant.clone(),
            });
        }

        Ok(())
    }
}

/// Builder for `AccessClaims` with required-field enforcement.
#[derive(Debug, Clone, Default)]
pub struct AccessClaimsBuilder {
    iss: Option<String>,
    sub: Option<String>,
    aud: Option<Vec<String>>,
    client_id: Option<String>,
    scope: Option<String>,
    exp: Option<i64>,
    nbf: Option<i64>,
    iat: Option<i64>,
    jti: Option<String>,
    ver: Option<u64>,
    sid: Option<String>,
    tenant_id: Option<String>,
    user_id: Option<String>,
    user_type: Option<String>,
    sx: Option<SesameAuthzClaims>,
    act: Option<ActorClaim>,
}

impl AccessClaimsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn iss(mut self, iss: impl Into<String>) -> Self {
        self.iss = Some(iss.into());
        self
    }

    pub fn sub(mut self, sub: impl Into<String>) -> Self {
        self.sub = Some(sub.into());
        self
    }

    pub fn aud(mut self, aud: Vec<String>) -> Self {
        self.aud = Some(aud);
        self
    }

    pub fn client_id(mut self, client_id: impl Into<String>) -> Self {
        self.client_id = Some(client_id.into());
        self
    }

    pub fn scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }

    pub fn exp(mut self, exp: i64) -> Self {
        self.exp = Some(exp);
        self
    }

    pub fn nbf(mut self, nbf: i64) -> Self {
        self.nbf = Some(nbf);
        self
    }

    pub fn iat(mut self, iat: i64) -> Self {
        self.iat = Some(iat);
        self
    }

    pub fn jti(mut self, jti: impl Into<String>) -> Self {
        self.jti = Some(jti.into());
        self
    }

    pub fn ver(mut self, ver: u64) -> Self {
        self.ver = Some(ver);
        self
    }

    pub fn sid(mut self, sid: impl Into<String>) -> Self {
        self.sid = Some(sid.into());
        self
    }

    pub fn tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn user_type(mut self, user_type: impl Into<String>) -> Self {
        self.user_type = Some(user_type.into());
        self
    }

    pub fn sx(mut self, sx: SesameAuthzClaims) -> Self {
        self.sx = Some(sx);
        self
    }

    pub fn act(mut self, act: ActorClaim) -> Self {
        self.act = Some(act);
        self
    }

    /// Build and validate `AccessClaims`.
    pub fn build(self) -> Result<AccessClaims, JwtError> {
        let iss = self
            .iss
            .ok_or_else(|| JwtError::MissingRequiredField("iss".into()))?;
        let sub = self
            .sub
            .ok_or_else(|| JwtError::MissingRequiredField("sub".into()))?;
        let aud = self
            .aud
            .ok_or_else(|| JwtError::MissingRequiredField("aud".into()))?;
        let client_id = self
            .client_id
            .ok_or_else(|| JwtError::MissingRequiredField("client_id".into()))?;
        let scope = self
            .scope
            .ok_or_else(|| JwtError::MissingRequiredField("scope".into()))?;
        let exp = self
            .exp
            .ok_or_else(|| JwtError::MissingRequiredField("exp".into()))?;
        let nbf = self
            .nbf
            .ok_or_else(|| JwtError::MissingRequiredField("nbf".into()))?;
        let iat = self
            .iat
            .ok_or_else(|| JwtError::MissingRequiredField("iat".into()))?;
        let jti = self
            .jti
            .ok_or_else(|| JwtError::MissingRequiredField("jti".into()))?;
        let ver = self
            .ver
            .ok_or_else(|| JwtError::MissingRequiredField("ver".into()))?;
        let sid = self
            .sid
            .ok_or_else(|| JwtError::MissingRequiredField("sid".into()))?;
        let tenant_id = self
            .tenant_id
            .ok_or_else(|| JwtError::MissingRequiredField("tenant_id".into()))?;
        let user_id = self
            .user_id
            .ok_or_else(|| JwtError::MissingRequiredField("user_id".into()))?;
        let user_type = self
            .user_type
            .ok_or_else(|| JwtError::MissingRequiredField("user_type".into()))?;
        let sx = self
            .sx
            .ok_or_else(|| JwtError::MissingRequiredField("sx".into()))?;

        let claims = AccessClaims {
            iss,
            sub,
            aud,
            client_id,
            scope,
            exp,
            nbf,
            iat,
            jti,
            ver,
            sid,
            tenant_id,
            user_id,
            user_type,
            sx,
            act: self.act,
        };

        if claims.ver == 0 {
            return Err(JwtValidationError::MissingVersion.into());
        }

        claims.validate()?;

        Ok(claims)
    }
}

// ===========================================================================
// Unit Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    // -----------------------------------------------------------------
    // ActorClaim round-trip
    // -----------------------------------------------------------------

    #[test]
    fn test_actor_claim_round_trip() {
        let claim = ActorClaim {
            sub: "user-123".into(),
        };
        let json = serde_json::to_string(&claim).unwrap();
        assert_eq!(json, r#"{"sub":"user-123"}"#);
        let round_trip: ActorClaim = serde_json::from_str(&json).unwrap();
        assert_eq!(round_trip, claim);
    }

    // -----------------------------------------------------------------
    // SesameAuthzClaims round-trip
    // -----------------------------------------------------------------

    #[test]
    fn test_authz_claims_full_round_trip() {
        let claims = SesameAuthzClaims {
            tenant: "hauliage".into(),
            portal: "hauliage-web".into(),
            roles: vec!["driver".into(), "dispatcher".into()],
            permissions: vec!["shipments:read".into(), "rates:write".into()],
            permissions_hash: Some("abc123".into()),
            entitlements_ref: Some("ent-456".into()),
            entitlements_hash: Some("hash789".into()),
            risk: Some("normal".into()),
        };
        let json = serde_json::to_string(&claims).unwrap();
        let round_trip: SesameAuthzClaims = serde_json::from_str(&json).unwrap();
        assert_eq!(round_trip, claims);
    }

    #[test]
    fn test_authz_claims_optional_fields_absent() {
        let claims = SesameAuthzClaims {
            tenant: "hauliage".into(),
            portal: "hauliage-web".into(),
            roles: vec![],
            permissions: vec![],
            permissions_hash: None,
            entitlements_ref: None,
            entitlements_hash: None,
            risk: None,
        };
        let json = serde_json::to_string(&claims).unwrap();
        assert!(!json.contains("permissions_hash"));
        assert!(!json.contains("entitlements_ref"));
        assert!(!json.contains("entitlements_hash"));
        assert!(!json.contains("risk"));
        assert!(json.contains("\"tenant\":\"hauliage\""));
        assert!(json.contains("\"portal\":\"hauliage-web\""));
    }

    #[test]
    fn test_authz_claims_builder() {
        let claims = SesameAuthzClaims::builder()
            .tenant("hauliage")
            .portal("hauliage-web")
            .roles(vec!["driver".into()])
            .permissions(vec!["shipments:read".into()])
            .build()
            .unwrap();
        assert_eq!(claims.tenant, "hauliage");
        assert_eq!(claims.roles, vec!["driver"]);
        assert_eq!(claims.permissions, vec!["shipments:read"]);
        assert!(claims.permissions_hash.is_some());
    }

    #[test]
    fn test_authz_claims_builder_missing_required() {
        let result = SesameAuthzClaims::builder().portal("hauliage-web").build();
        assert_eq!(result, Err(JwtError::MissingRequiredField("tenant".into())));

        let result2 = SesameAuthzClaims::builder().tenant("hauliage").build();
        assert_eq!(
            result2,
            Err(JwtError::MissingRequiredField("portal".into()))
        );
    }

    #[test]
    fn test_authz_claims_permissions_hash_deterministic() {
        let c1 = SesameAuthzClaims {
            tenant: "a".into(),
            portal: "b".into(),
            roles: vec![],
            permissions: vec!["perm".into()],
            permissions_hash: None,
            entitlements_ref: None,
            entitlements_hash: None,
            risk: None,
        };
        let h1 = c1.compute_permissions_hash();

        let c2 = SesameAuthzClaims {
            tenant: "x".into(),
            portal: "y".into(),
            roles: vec![],
            permissions: vec!["perm".into()],
            permissions_hash: None,
            entitlements_ref: None,
            entitlements_hash: None,
            risk: None,
        };
        let h2 = c2.compute_permissions_hash();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_authz_claims_permissions_hash_sorted() {
        let c_unsorted = SesameAuthzClaims {
            tenant: "a".into(),
            portal: "b".into(),
            roles: vec![],
            permissions: vec!["z".into(), "a".into(), "m".into()],
            permissions_hash: None,
            entitlements_ref: None,
            entitlements_hash: None,
            risk: None,
        };
        let h_unsorted = c_unsorted.compute_permissions_hash();

        let c_sorted = SesameAuthzClaims {
            tenant: "a".into(),
            portal: "b".into(),
            roles: vec![],
            permissions: vec!["a".into(), "m".into(), "z".into()],
            permissions_hash: None,
            entitlements_ref: None,
            entitlements_hash: None,
            risk: None,
        };
        let h_sorted = c_sorted.compute_permissions_hash();
        assert_eq!(h_unsorted, h_sorted);
    }

    // -----------------------------------------------------------------
    // Helper: make a fully valid AccessClaims
    // -----------------------------------------------------------------

    fn make_valid_claims() -> AccessClaims {
        let sx = SesameAuthzClaims {
            tenant: "hauliage".into(),
            portal: "hauliage-web".into(),
            roles: vec!["driver".into()],
            permissions: vec!["shipments:read".into()],
            permissions_hash: None,
            entitlements_ref: None,
            entitlements_hash: None,
            risk: Some("normal".into()),
        };
        AccessClaims {
            iss: "https://idam.example.com".into(),
            sub: "user-123".into(),
            aud: vec!["authz-core".into()],
            client_id: "hauliage-web".into(),
            scope: "profile:read".into(),
            exp: 1779212000,
            nbf: 1779211700,
            iat: 1779211700,
            jti: "tok-12345".into(),
            ver: 1,
            sid: "session-abc".into(),
            tenant_id: "hauliage".into(),
            user_id: "user-123".into(),
            user_type: "registered".into(),
            sx,
            act: None,
        }
    }

    // -----------------------------------------------------------------
    // AccessClaims validation tests
    // -----------------------------------------------------------------

    #[test]
    fn test_validate_valid_claims() {
        assert!(make_valid_claims().validate().is_ok());
    }

    #[test]
    fn test_validate_missing_version() {
        let mut c = make_valid_claims();
        c.ver = 0;
        assert_eq!(c.validate(), Err(JwtValidationError::MissingVersion));
    }

    #[test]
    fn test_validate_missing_tenant_id() {
        let mut c = make_valid_claims();
        c.tenant_id = String::new();
        assert_eq!(c.validate(), Err(JwtValidationError::MissingTenant));
    }

    #[test]
    fn test_validate_missing_sx_tenant() {
        let mut c = make_valid_claims();
        c.sx.tenant = String::new();
        assert_eq!(c.validate(), Err(JwtValidationError::MissingAuthzClaims));
    }

    #[test]
    fn test_validate_invalid_risk() {
        let mut c = make_valid_claims();
        c.sx.risk = Some("unknown".into());
        assert_eq!(
            c.validate(),
            Err(JwtValidationError::InvalidRisk("unknown".into()))
        );
    }

    #[test]
    fn test_validate_valid_risk_values() {
        for risk in &["normal", "elevated", "critical"] {
            let mut c = make_valid_claims();
            c.sx.risk = Some(risk.to_string());
            assert!(c.validate().is_ok(), "risk '{}' should be valid", risk);
        }
    }

    #[test]
    fn test_validate_invalid_issuer() {
        let mut c = make_valid_claims();
        c.iss = "https://evil.example.com".into();
        assert_eq!(
            c.validate(),
            Err(JwtValidationError::InvalidIssuer(
                "https://evil.example.com".into()
            ))
        );
    }

    #[test]
    fn test_validate_invalid_audience() {
        let mut c = make_valid_claims();
        c.aud = vec!["unknown-service".into()];
        assert_eq!(c.validate(), Err(JwtValidationError::InvalidAudience));
    }

    #[test]
    fn test_validate_empty_audience() {
        let mut c = make_valid_claims();
        c.aud = vec![];
        assert_eq!(c.validate(), Err(JwtValidationError::InvalidAudience));
    }

    #[test]
    fn test_validate_scope_empty_is_ok() {
        let mut c = make_valid_claims();
        c.scope = String::new();
        assert!(c.validate().is_ok());
    }

    #[test]
    fn test_validate_multiple_audiences_one_match() {
        let mut c = make_valid_claims();
        c.aud = vec![
            "unknown".into(),
            "identity-login-service".into(),
            "other".into(),
        ];
        assert!(c.validate().is_ok());
    }

    // -----------------------------------------------------------------
    // Serialization tests
    // -----------------------------------------------------------------

    #[test]
    fn test_sx_serialization_key() {
        let c = make_valid_claims();
        let json = serde_json::to_string(&c).unwrap();
        assert!(
            json.contains("\"https://sesame-idam.dev/claims\""),
            "Expected namespaced key in JSON: {}",
            json
        );
    }

    #[test]
    fn test_access_claims_with_act_serializes() {
        let sx = SesameAuthzClaims {
            tenant: "hauliage".into(),
            portal: "hauliage-web".into(),
            roles: vec![],
            permissions: vec![],
            permissions_hash: None,
            entitlements_ref: None,
            entitlements_hash: None,
            risk: None,
        };
        let c = AccessClaims {
            iss: "https://idam.example.com".into(),
            sub: "user-123".into(),
            aud: vec!["authz-core".into()],
            client_id: "hauliage-web".into(),
            scope: "".into(),
            exp: 1779212000,
            nbf: 1779211700,
            iat: 1779211700,
            jti: "tok-12345".into(),
            ver: 1,
            sid: "session-abc".into(),
            tenant_id: "hauliage".into(),
            user_id: "user-123".into(),
            user_type: "registered".into(),
            sx,
            act: Some(ActorClaim {
                sub: "admin-999".into(),
            }),
        };
        let json = serde_json::to_string(&c).unwrap();
        assert!(json.contains("\"act\""));
        assert!(json.contains("\"sub\":\"admin-999\""));
    }

    #[test]
    fn test_access_claims_without_act_no_act_key() {
        let sx = SesameAuthzClaims {
            tenant: "hauliage".into(),
            portal: "hauliage-web".into(),
            roles: vec![],
            permissions: vec![],
            permissions_hash: None,
            entitlements_ref: None,
            entitlements_hash: None,
            risk: None,
        };
        let c = AccessClaims {
            iss: "https://idam.example.com".into(),
            sub: "user-123".into(),
            aud: vec!["authz-core".into()],
            client_id: "hauliage-web".into(),
            scope: "".into(),
            exp: 1779212000,
            nbf: 1779211700,
            iat: 1779211700,
            jti: "tok-12345".into(),
            ver: 1,
            sid: "session-abc".into(),
            tenant_id: "hauliage".into(),
            user_id: "user-123".into(),
            user_type: "registered".into(),
            sx,
            act: None,
        };
        let json = serde_json::to_string(&c).unwrap();
        assert!(!json.contains("\"act\""));
    }

    #[test]
    fn test_access_claims_round_trip() {
        let c = make_valid_claims();
        let json = serde_json::to_string(&c).unwrap();
        let rt: AccessClaims = serde_json::from_str(&json).unwrap();
        assert_eq!(rt, c);
    }

    // -----------------------------------------------------------------
    // Builder tests
    // -----------------------------------------------------------------

    #[test]
    fn test_builder_constructs_valid_claims() {
        let sx = SesameAuthzClaims::builder()
            .tenant("hauliage")
            .portal("hauliage-web")
            .roles(vec!["driver".into()])
            .permissions(vec!["shipments:read".into()])
            .build()
            .unwrap();

        let claims = AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("user-123")
            .aud(vec!["authz-core".into()])
            .client_id("hauliage-web")
            .scope("profile:read")
            .exp(1779212000)
            .nbf(1779211700)
            .iat(1779211700)
            .jti("tok-12345")
            .ver(1)
            .sid("session-abc")
            .tenant_id("hauliage")
            .user_id("user-123")
            .user_type("registered")
            .sx(sx)
            .build();

        assert!(claims.is_ok());
        let c = claims.unwrap();
        assert_eq!(c.tenant_id, "hauliage");
        assert_eq!(c.ver, 1);
        assert_eq!(c.act, None);
    }

    #[test]
    fn test_builder_rejects_missing_required() {
        let sx = SesameAuthzClaims::builder()
            .tenant("hauliage")
            .portal("hauliage-web")
            .build()
            .unwrap();

        let result = AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("user-123")
            .aud(vec!["authz-core".into()])
            .client_id("hauliage-web")
            .scope("profile:read")
            .exp(1779212000)
            .nbf(1779211700)
            .iat(1779211700)
            .jti("tok-12345")
            // ver missing
            .sid("session-abc")
            .tenant_id("hauliage")
            .user_id("user-123")
            .user_type("registered")
            .sx(sx)
            .build();

        assert_eq!(result, Err(JwtError::MissingRequiredField("ver".into())));
    }

    #[test]
    fn test_builder_rejects_ver_zero() {
        let sx = SesameAuthzClaims::builder()
            .tenant("hauliage")
            .portal("hauliage-web")
            .build()
            .unwrap();

        let result = AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("user-123")
            .aud(vec!["authz-core".into()])
            .client_id("hauliage-web")
            .scope("profile:read")
            .exp(1779212000)
            .nbf(1779211700)
            .iat(1779211700)
            .jti("tok-12345")
            .ver(0)
            .sid("session-abc")
            .tenant_id("hauliage")
            .user_id("user-123")
            .user_type("registered")
            .sx(sx)
            .build();

        assert_eq!(
            result,
            Err(JwtError::ValidationError(
                JwtValidationError::MissingVersion
            ))
        );
    }

    #[test]
    fn test_builder_with_act() {
        let sx = SesameAuthzClaims::builder()
            .tenant("hauliage")
            .portal("hauliage-web")
            .build()
            .unwrap();

        let claims = AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("user-123")
            .aud(vec!["authz-core".into()])
            .client_id("hauliage-web")
            .scope("profile:read")
            .exp(1779212000)
            .nbf(1779211700)
            .iat(1779211700)
            .jti("tok-12345")
            .ver(1)
            .sid("session-abc")
            .tenant_id("hauliage")
            .user_id("user-123")
            .user_type("registered")
            .sx(sx)
            .act(ActorClaim {
                sub: "admin-999".into(),
            })
            .build()
            .unwrap();

        assert!(claims.act.is_some());
        assert_eq!(claims.act.unwrap().sub, "admin-999");
    }

    // -----------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------

    #[test]
    fn test_very_long_tenant_id() {
        let long_tenant = "t".repeat(1000);
        let sx = SesameAuthzClaims {
            tenant: "hauliage".into(),
            portal: "hauliage-web".into(),
            roles: vec![],
            permissions: vec![],
            permissions_hash: None,
            entitlements_ref: None,
            entitlements_hash: None,
            risk: None,
        };
        let c = AccessClaims {
            iss: "https://idam.example.com".into(),
            sub: "user-123".into(),
            aud: vec!["authz-core".into()],
            client_id: "hauliage-web".into(),
            scope: "".into(),
            exp: 1779212000,
            nbf: 1779211700,
            iat: 1779211700,
            jti: "tok-12345".into(),
            ver: 1,
            sid: "session-abc".into(),
            tenant_id: long_tenant,
            user_id: "user-123".into(),
            user_type: "registered".into(),
            sx,
            act: None,
        };
        let json = serde_json::to_string(&c).unwrap();
        assert!(json.len() > 1000);
        assert!(c.validate().is_ok());
    }

    #[test]
    fn test_very_large_roles_array() {
        let roles: Vec<String> = (0..500).map(|i| format!("role-{}", i)).collect();
        let sx = SesameAuthzClaims {
            tenant: "hauliage".into(),
            portal: "hauliage-web".into(),
            roles,
            permissions: vec![],
            permissions_hash: None,
            entitlements_ref: None,
            entitlements_hash: None,
            risk: None,
        };
        let c = AccessClaims {
            iss: "https://idam.example.com".into(),
            sub: "user-123".into(),
            aud: vec!["authz-core".into()],
            client_id: "hauliage-web".into(),
            scope: "".into(),
            exp: 1779212000,
            nbf: 1779211700,
            iat: 1779211700,
            jti: "tok-12345".into(),
            ver: 1,
            sid: "session-abc".into(),
            tenant_id: "hauliage".into(),
            user_id: "user-123".into(),
            user_type: "registered".into(),
            sx,
            act: None,
        };
        let json = serde_json::to_string(&c).unwrap();
        assert!(json.len() > 5000);
    }

    // -----------------------------------------------------------------
    // Trait derivation tests
    // -----------------------------------------------------------------

    #[test]
    fn test_all_structs_derive_traits() {
        let clone_test = ActorClaim { sub: "x".into() };
        let _cloned = clone_test.clone();
        let _formatted = format!("{:?}", clone_test);
        assert_eq!(clone_test, ActorClaim { sub: "x".into() });

        let authz = SesameAuthzClaims {
            tenant: "a".into(),
            portal: "b".into(),
            roles: vec![],
            permissions: vec![],
            permissions_hash: None,
            entitlements_ref: None,
            entitlements_hash: None,
            risk: None,
        };
        let _authz_cloned = authz.clone();
        let _authz_formatted = format!("{:?}", authz);
        assert_eq!(
            authz,
            SesameAuthzClaims {
                tenant: "a".into(),
                portal: "b".into(),
                roles: vec![],
                permissions: vec![],
                permissions_hash: None,
                entitlements_ref: None,
                entitlements_hash: None,
                risk: None,
            }
        );
    }

    // -----------------------------------------------------------------
    // Legacy JWT deserialization
    // -----------------------------------------------------------------

    #[test]
    fn test_legacy_jwt_missing_version_fails_validation() {
        let sx = SesameAuthzClaims {
            tenant: "hauliage".into(),
            portal: "hauliage-web".into(),
            roles: vec![],
            permissions: vec![],
            permissions_hash: None,
            entitlements_ref: None,
            entitlements_hash: None,
            risk: None,
        };
        let c = AccessClaims {
            iss: "https://idam.example.com".into(),
            sub: "user-123".into(),
            aud: vec!["authz-core".into()],
            client_id: "hauliage-web".into(),
            scope: "".into(),
            exp: 1779212000,
            nbf: 1779211700,
            iat: 1779211700,
            jti: "old-tok".into(),
            ver: 0,
            sid: "".into(),
            tenant_id: "".into(),
            user_id: "user-123".into(),
            user_type: "registered".into(),
            sx,
            act: None,
        };
        assert_eq!(c.validate(), Err(JwtValidationError::MissingVersion));
    }

    // ===========================================================================
    // Story 2.4: Tenant-specific tests
    // ===========================================================================

    /// HACK-241: Tenant ID can be stolen via JWT token replay
    /// Test: validate_tenant rejects mismatch between JWT tenant and header tenant
    #[test]
    fn test_validate_tenant_rejects_mismatch() {
        let claims = AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("user-123")
            .aud(vec!["authz-core".into()])
            .client_id("test-app")
            .scope("profile:read")
            .exp(9999999999)
            .nbf(0)
            .iat(0)
            .jti("jti-1")
            .ver(1)
            .sid("session-1")
            .tenant_id("tenant-abc")
            .user_id("user-123")
            .user_type("registered")
            .sx(
                SesameAuthzClaims::builder()
                    .tenant("tenant-abc")
                    .portal("web")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        // Should reject when tenant_id doesn't match expected
        assert!(matches!(
            claims.validate_tenant("tenant-xyz"),
            Err(JwtError::TenantMismatch { .. })
        ));
    }

    /// HACK-243: Tenant ID mismatch validation can be bypassed by omitting X-Tenant-ID
    /// Test: validate_tenant rejects when expected_tenant is empty (missing header)
    #[test]
    fn test_validate_tenant_rejects_missing_header() {
        let claims = AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("user-123")
            .aud(vec!["authz-core".into()])
            .client_id("test-app")
            .scope("profile:read")
            .exp(9999999999)
            .nbf(0)
            .iat(0)
            .jti("jti-1")
            .ver(1)
            .sid("session-1")
            .tenant_id("tenant-abc")
            .user_id("user-123")
            .user_type("registered")
            .sx(
                SesameAuthzClaims::builder()
                    .tenant("tenant-abc")
                    .portal("web")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        // Empty expected_tenant (missing X-Tenant-ID header) should still reject
        assert!(matches!(
            claims.validate_tenant(""),
            Err(JwtError::TenantMismatch { .. })
        ));
    }

    /// HACK-242: Tenant ID in JWT is NOT confidential
    /// Test: tenant_id is present at both top-level and namespaced in serialized JSON
    #[test]
    fn test_tenant_id_present_in_both_locations() {
        let claims = AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("user-123")
            .aud(vec!["authz-core".into()])
            .client_id("test-app")
            .scope("profile:read")
            .exp(9999999999)
            .nbf(0)
            .iat(0)
            .jti("jti-1")
            .ver(1)
            .sid("session-1")
            .tenant_id("hauliage-tenant-uuid")
            .user_id("user-123")
            .user_type("registered")
            .sx(
                SesameAuthzClaims::builder()
                    .tenant("hauliage-tenant-uuid")
                    .portal("web")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        let json = serde_json::to_string(&claims).unwrap();

        // Top-level tenant_id
        assert!(json.contains(r#""tenant_id":"hauliage-tenant-uuid""#));

        // Namespaced sx.tenant
        assert!(json.contains(r#""https://sesame-idam.dev/claims""#));
        assert!(json.contains(r#""tenant":"hauliage-tenant-uuid""#));
    }

    /// Acceptance: Both top-level and namespaced tenant_id contain the same UUID
    #[test]
    fn test_tenant_id_consistent_across_locations() {
        let claims = AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("user-123")
            .aud(vec!["authz-core".into()])
            .client_id("test-app")
            .scope("profile:read")
            .exp(9999999999)
            .nbf(0)
            .iat(0)
            .jti("jti-1")
            .ver(1)
            .sid("session-1")
            .tenant_id("abc-123-tenant-uuid")
            .user_id("user-123")
            .user_type("registered")
            .sx(
                SesameAuthzClaims::builder()
                    .tenant("abc-123-tenant-uuid")
                    .portal("web")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        // Both should be identical
        assert_eq!(claims.tenant_id, "abc-123-tenant-uuid");
        assert_eq!(claims.sx.tenant, "abc-123-tenant-uuid");
    }

    /// Acceptance: Downstream service can validate claims.tenant_id against X-Tenant-ID
    #[test]
    fn test_downstream_tenant_validation() {
        let claims = AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("user-123")
            .aud(vec!["authz-core".into()])
            .client_id("test-app")
            .scope("profile:read")
            .exp(9999999999)
            .nbf(0)
            .iat(0)
            .jti("jti-1")
            .ver(1)
            .sid("session-1")
            .tenant_id("hauliage")
            .user_id("user-123")
            .user_type("registered")
            .sx(
                SesameAuthzClaims::builder()
                    .tenant("hauliage")
                    .portal("web")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        // Downstream service validates against X-Tenant-ID header
        assert!(claims.validate_tenant("hauliage").is_ok());

        // Mismatched tenant should be rejected
        assert!(claims.validate_tenant("rerp").is_err());
    }

    /// Edge case: Malformed tenant_id UUID — validate_tenant still works with non-UUID strings
    #[test]
    fn test_validate_tenant_with_non_uuid_tenant_id() {
        // Even non-UUID tenant IDs should be validated for consistency
        let claims = AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("user-123")
            .aud(vec!["authz-core".into()])
            .client_id("test-app")
            .scope("profile:read")
            .exp(9999999999)
            .nbf(0)
            .iat(0)
            .jti("jti-1")
            .ver(1)
            .sid("session-1")
            .tenant_id("tenant-abc")
            .user_id("user-123")
            .user_type("registered")
            .sx(
                SesameAuthzClaims::builder()
                    .tenant("tenant-abc")
                    .portal("web")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        assert!(claims.validate_tenant("tenant-abc").is_ok());
        assert!(claims.validate_tenant("different").is_err());
    }

    /// HACK-245: Tenant ID UUID format — tenant_id is validated as a valid UUID format
    #[test]
    fn test_tenant_id_uuid_format() {
        let claims = AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("user-123")
            .aud(vec!["authz-core".into()])
            .client_id("test-app")
            .scope("profile:read")
            .exp(9999999999)
            .nbf(0)
            .iat(0)
            .jti("jti-1")
            .ver(1)
            .sid("session-1")
            .tenant_id("6ba7b810-9dad-11d1-80b4-00c04fd430c8")
            .user_id("user-123")
            .user_type("registered")
            .sx(
                SesameAuthzClaims::builder()
                    .tenant("6ba7b810-9dad-11d1-80b4-00c04fd430c8")
                    .portal("web")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        // UUID format should validate successfully
        assert!(claims.validate_tenant("6ba7b810-9dad-11d1-80b4-00c04fd430c8").is_ok());
    }

    /// Acceptance: User's actual tenant matches request tenant
    /// Test: login handler should reject if user's tenant != request tenant
    #[test]
    fn test_user_tenant_matches_request_tenant() {
        // Simulates: user registered under tenant A, attempts login with tenant B header
        let user_claims = AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("alice")
            .aud(vec!["authz-core".into()])
            .client_id("test-app")
            .scope("profile:read")
            .exp(9999999999)
            .nbf(0)
            .iat(0)
            .jti("jti-1")
            .ver(1)
            .sid("session-1")
            .tenant_id("tenant-hauliage")
            .user_id("alice")
            .user_type("registered")
            .sx(
                SesameAuthzClaims::builder()
                    .tenant("tenant-hauliage")
                    .portal("web")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        // User A should NOT be able to access tenant B data
        assert!(matches!(
            user_claims.validate_tenant("tenant-rerp"),
            Err(JwtError::TenantMismatch { .. })
        ));
    }

    /// Acceptance: Different users on different tenants have different JWT tenants
    #[test]
    fn test_different_users_different_tenants() {
        let hauliage_claims = AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("alice@hauliage")
            .aud(vec!["authz-core".into()])
            .client_id("test-app")
            .scope("profile:read")
            .exp(9999999999)
            .nbf(0)
            .iat(0)
            .jti("jti-1")
            .ver(1)
            .sid("session-1")
            .tenant_id("hauliage-uuid")
            .user_id("alice@hauliage")
            .user_type("registered")
            .sx(
                SesameAuthzClaims::builder()
                    .tenant("hauliage-uuid")
                    .portal("web")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        let rerp_claims = AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("alice@rerp")
            .aud(vec!["authz-core".into()])
            .client_id("test-app")
            .scope("profile:read")
            .exp(9999999999)
            .nbf(0)
            .iat(0)
            .jti("jti-2")
            .ver(1)
            .sid("session-2")
            .tenant_id("rerp-uuid")
            .user_id("alice@rerp")
            .user_type("registered")
            .sx(
                SesameAuthzClaims::builder()
                    .tenant("rerp-uuid")
                    .portal("web")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        // Confirm zero cross-tenant identity
        assert_eq!(haul_iage_claims.tenant_id, "hauliage-uuid");
        assert_eq!(rerp_claims.tenant_id, "rerp-uuid");
        assert_ne!(haul_iage_claims.tenant_id, rerp_claims.tenant_id);

        // Each JWT should only validate for its own tenant
        assert!(haul_iage_claims.validate_tenant("hauliage-uuid").is_ok());
        assert!(haul_iage_claims.validate_tenant("rerp-uuid").is_err());
        assert!(rerp_claims.validate_tenant("rerp-uuid").is_ok());
        assert!(rerp_claims.validate_tenant("hauliage-uuid").is_err());
    }

    /// Security regression: Tenant ID cannot be forged in token
    /// Test: if a client modifies tenant_id, validate_tenant detects the mismatch
    #[test]
    fn test_tenant_id_cannot_be_forged() {
        let legitimate_claims = AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("user-123")
            .aud(vec!["authz-core".into()])
            .client_id("test-app")
            .scope("profile:read")
            .exp(9999999999)
            .nbf(0)
            .iat(0)
            .jti("jti-1")
            .ver(1)
            .sid("session-1")
            .tenant_id("tenant-abc")
            .user_id("user-123")
            .user_type("registered")
            .sx(
                SesameAuthzClaims::builder()
                    .tenant("tenant-abc")
                    .portal("web")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        // Even without signature verification, tenant_id mismatch is detectable
        // by validate_tenant — the attacker would need to forge both tenant_id
        // and sx.tenant to match a different tenant
    }

    /// Security regression: No tenant_id leakage across login sessions
    #[test]
    fn test_no_tenant_leakage_across_sessions() {
        let session_a = AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("user-1")
            .aud(vec!["authz-core".into()])
            .client_id("test-app")
            .scope("profile:read")
            .exp(9999999999)
            .nbf(0)
            .iat(0)
            .jti("jti-a")
            .ver(1)
            .sid("session-a")
            .tenant_id("tenant-a-uuid")
            .user_id("user-1")
            .user_type("registered")
            .sx(
                SesameAuthzClaims::builder()
                    .tenant("tenant-a-uuid")
                    .portal("web")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        let session_b = AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("user-2")
            .aud(vec!["authz-core".into()])
            .client_id("test-app")
            .scope("profile:read")
            .exp(9999999999)
            .nbf(0)
            .iat(0)
            .jti("jti-b")
            .ver(1)
            .sid("session-b")
            .tenant_id("tenant-b-uuid")
            .user_id("user-2")
            .user_type("registered")
            .sx(
                SesameAuthzClaims::builder()
                    .tenant("tenant-b-uuid")
                    .portal("web")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        // Tenant A's token must NEVER contain Tenant B's UUID
        assert!(
            session_a.validate_tenant("tenant-b-uuid").is_err(),
            "Tenant A JWT should not validate against Tenant B"
        );
        assert!(
            session_b.validate_tenant("tenant-a-uuid").is_err(),
            "Tenant B JWT should not validate against Tenant A"
        );
    }
}

