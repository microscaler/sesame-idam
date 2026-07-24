//! JWT claim structures for Sesame-IDAM.
//!
//! Core types: `ActorClaim`, `EntitlementsSnapshot`, `SesameAuthzClaims`, `AccessClaims`.
//! Validation enums: `JwtValidationError`, `JwtError`.
//!
//! ## PII Removal (Story 2.3)
//!
//! PII fields (email, `email_verified`, `phone_number`, `phone_verified`, `first_name`,
//! `last_name`, name, `preferred_username`) are REMOVED from the JWT access token.
//! Consumers that need PII must fetch them from GET /api/v1/identity/users/me.
//!
//! ## Entitlements Reference Pattern
//!
//! The full permissions array is replaced with:
//! - `entitlements_ref` — deterministic UUID-based reference for Redis lookup
//! - `entitlements_hash` — SHA-256 hash of canonical JSON of the entitlements snapshot
//!
//! Entitlements snapshots are stored in Redis with key `entitlements:{tenant_id}:{entitlements_ref}`
//! and TTL 30-300 seconds. Consumers verify the hash before trusting cached snapshots.
//!
//! ## Security (Story 2.3)
//!
//! - Entitlements snapshots are keyed by tenant to prevent cross-tenant bleed
//! - Hash verification is mandatory after every Redis cache fetch
//! - Entitlements refs are deterministic (for caching consistency) but require Redis access

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::dpop::DpopConfirmation;

// ===========================================================================
// Private helpers
// ===========================================================================

/// Canonical (sorted-keys) JSON representation for deterministic hashing.
///
/// HACK-207: All hash computations MUST use canonical JSON with sorted
/// keys and no whitespace so different implementations produce identical
/// digests. `serde_json::to_string` does NOT sort keys by default.
pub(super) fn canonical_serialize<T: serde::ser::Serialize>(value: &T) -> String {
    fn sort_value(v: &serde_json::Value) -> serde_json::Value {
        match v {
            serde_json::Value::Object(map) => {
                let sorted: serde_json::Map<String, serde_json::Value> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), sort_value(v)))
                    .collect();
                serde_json::Value::Object(sorted)
            }
            serde_json::Value::Array(arr) => {
                let sorted: Vec<serde_json::Value> = arr.iter().map(sort_value).collect();
                serde_json::Value::Array(sorted)
            }
            other => other.clone(),
        }
    }

    let json_val = match serde_json::to_value(value) {
        Ok(v) => v,
        Err(_) => return String::new(),
    };

    let sorted = sort_value(&json_val);
    serde_json::to_string(&sorted).unwrap_or_default()
}

/// Custom namespace UUID for deterministic entitlements refs.
pub(super) fn entitlements_namespace() -> Uuid {
    let hash = Sha256::digest(b"sesame-idam-entitlements-namespace");
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&hash[..16]);
    // Set version to 5
    bytes[6] = (bytes[6] & 0x0F) | 0x50;
    // Set variant to RFC 4122
    bytes[8] = (bytes[8] & 0x3F) | 0x80;
    Uuid::from_bytes(bytes)
}

// ===========================================================================
// Public structs and enums
// ===========================================================================

/// Entitlements snapshot stored in Redis.
/// This is the FULL ACL snapshot that replaces embedding permissions in the JWT.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntitlementsSnapshot {
    /// Version of the entitlements (must match JWT `ver` claim)
    pub version: u64,
    /// List of permission strings (e.g., ["org:admin", "billing:read"])
    pub permissions: Vec<String>,
    /// List of role names (e.g., ["admin", "billing-viewer"])
    pub roles: Vec<String>,
    /// Tenant ID - critical for multi-tenant isolation
    pub tenant: String,
    /// SHA-256 hash of the canonical JSON representation of this struct
    /// Format: "sha256:<64 hex chars>"
    pub hash: String,
}

/// RFC 8693 delegation actor claim.
/// Present when a token is the result of token exchange (act claim).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActorClaim {
    /// Subject of the actor (the user acting on behalf of the token holder)
    pub sub: String,
}

/// Namespaced authorization data — the `sx` field in JWT payloads.
///
/// PII fields have been REMOVED per Story 2.3. Consumers that need PII must
/// fetch from GET /api/v1/identity/users/me.
///
/// The full permissions array is replaced with `entitlements_ref` and
/// `entitlements_hash` for compact tokens and cache-based resolution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SesameAuthzClaims {
    /// Tenant ID (hard-segment isolation boundary)
    pub tenant: String,
    /// Portal / application name the user is authenticated into
    pub portal: String,
    /// Coarse-grained role names (e.g., ["admin", "billing-viewer"])
    pub roles: Vec<String>,
    /// Coarse-grained permission hints (bounded set for common path).
    /// For fine-grained checks, use `entitlements_ref` to fetch full ACL from cache.
    pub permissions: Vec<String>,
    /// Deterministic reference to the full ACL snapshot in Redis.
    /// Format: "ent_<uuid>" where UUID is `v5(user_id:org_id:version:tenant_id)`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entitlements_ref: Option<String>,
    /// SHA-256 hash of the entitlements snapshot (canonical JSON).
    /// Format: "sha256:<64 hex chars>".
    /// Used to verify cached snapshots have not been tampered with.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entitlements_hash: Option<String>,
    /// Risk level for the token (normal, elevated, critical).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk: Option<String>,
}

/// Top-level JWT claim structure.
/// Standard JWT claims at top level, authz claims namespaced under <https://sesame-idam.dev/claims>.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccessClaims {
    // Standard claims
    pub iss: String,
    pub sub: String,
    pub aud: Vec<String>,
    pub client_id: String,
    pub scope: String,
    pub exp: i64,
    pub nbf: i64,
    pub iat: i64,
    pub jti: String,
    // Version claims
    pub ver: u64,
    pub sid: String,
    // Tenancy
    pub tenant_id: String,
    pub user_id: String,
    pub user_type: String,
    /// Active organization workspace (Sesame org id). Omitted until user creates or joins an org.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_id: Option<String>,
    // Namespaced authz claims
    #[serde(rename = "https://sesame-idam.dev/claims")]
    pub sx: SesameAuthzClaims,
    // Optional delegation (RFC 8693)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act: Option<ActorClaim>,
    // DPoP confirmation (RFC 9449)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cnf: Option<DpopConfirmation>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JwtValidationError {
    InvalidIssuer,
    InvalidAudience,
    MissingVersion,
    MissingTenant,
    MissingAuthzClaims,
    InvalidRisk,
    InvalidTokenVersion,
    Expired,
    NotYetValid,
    SignatureInvalid,
    EntitlementsHashMismatch,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JwtError {
    Validation(JwtValidationError),
    MissingRequiredField(String),
    TenantMismatch { expected: String, actual: String },
}

impl std::fmt::Display for JwtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JwtError::Validation(e) => write!(f, "JWT validation error: {e:?}"),
            JwtError::MissingRequiredField(field) => write!(f, "Missing required field: {field}"),
            JwtError::TenantMismatch { expected, actual } => {
                write!(f, "Tenant mismatch: expected {expected}, got {actual}")
            }
        }
    }
}

// ===========================================================================
// AccessClaims impl block
// ===========================================================================

impl AccessClaims {
    /// Validate standard JWT claims (issuer, audience, version, tenant, risk).
    pub fn validate(&self) -> Result<(), JwtValidationError> {
        // Gate A6: expectations are environment config (JWT_ALLOWED_ISSUERS /
        // JWT_EXPECTED_AUDIENCES), not compile-time constants.
        if !super::helpers::allowed_issuers()
            .iter()
            .any(|i| i == &self.iss)
        {
            return Err(JwtValidationError::InvalidIssuer);
        }
        if !self.aud.iter().any(|a| {
            super::helpers::expected_audiences()
                .iter()
                .any(|e| e == a)
        }) {
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
        if let Some(risk) = &self.sx.risk {
            if !["normal", "elevated", "critical"].contains(&risk.as_str()) {
                return Err(JwtValidationError::InvalidRisk);
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn builder() -> crate::jwt::builders::AccessClaimsBuilder {
        crate::jwt::builders::AccessClaimsBuilder::new()
    }

    /// Serialize to canonical JSON string for hashing.
    #[must_use]
    pub fn to_canonical_json(&self) -> String {
        let value = serde_json::to_value(self).unwrap_or(serde_json::Value::Null);
        serde_json::to_string(&value).unwrap_or_default()
    }

    /// Serialize to compact JSON (for JWT payload).
    #[must_use]
    pub fn to_compact_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Get size of compact JSON payload in bytes.
    #[must_use]
    pub fn json_payload_size(&self) -> usize {
        self.to_compact_json().len()
    }

    /// Validate that the request's tenant matches the claims' tenant.
    ///
    /// Checks BOTH top-level `tenant_id` AND namespaced `sx.tenant` against
    /// the request's X-Tenant-ID header (HACK-241, HACK-243).
    ///
    /// # Requirements (HACK-243)
    /// - Returns `Err(JwtError::MissingRequiredField("X-Tenant-ID"))` if `request_tenant` is empty
    /// - Checks both `self.tenant_id` and `self.sx.tenant` — mismatch in either direction fails
    /// - MUST be called before any database query to prevent cross-tenant data leakage
    pub fn validate_tenant(&self, request_tenant: &str) -> Result<(), JwtError> {
        // HACK-243: Reject empty/missing X-Tenant-ID — never treat None as "no constraint"
        if request_tenant.is_empty() {
            return Err(JwtError::MissingRequiredField("X-Tenant-ID".to_string()));
        }
        // Check top-level tenant_id
        if self.tenant_id != request_tenant {
            return Err(JwtError::TenantMismatch {
                expected: self.tenant_id.clone(),
                actual: request_tenant.to_string(),
            });
        }
        // Check namespaced sx.tenant (HACK-241: both locations must match)
        if self.sx.tenant != request_tenant {
            return Err(JwtError::TenantMismatch {
                expected: self.sx.tenant.clone(),
                actual: request_tenant.to_string(),
            });
        }
        Ok(())
    }
}

// ===========================================================================
// SesameAuthzClaims impl block
// ===========================================================================

impl SesameAuthzClaims {
    #[must_use]
    pub fn new(
        tenant: String,
        portal: String,
        roles: Vec<String>,
        permissions: Vec<String>,
    ) -> Self {
        Self {
            tenant,
            portal,
            roles,
            permissions,
            entitlements_ref: None,
            entitlements_hash: None,
            risk: None,
        }
    }
}
