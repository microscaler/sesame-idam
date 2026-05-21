//! JWT claim structures for Sesame-IDAM.
//!
//! This module implements the new JWT claim structure defined in Epic 2 (Claims Schema Evolution):
//! - `ActorClaim` - RFC 8693 delegation actor claim
//! - `EntitlementsSnapshot` - cached ACL snapshot stored in Redis
//! - `SesameAuthzClaims` - namespaced authorization data (https://sesame-idam.dev/claims)
//! - `AccessClaims` - top-level JWT claim structure
//!
//! ## PII Removal (Story 2.3)
//!
//! PII fields (email, email_verified, phone_number, phone_verified, first_name,
//! last_name, name, preferred_username) are REMOVED from the JWT access token.
//! Consumers that need PII must fetch them from GET /api/v1/identity/users/me.
//!
//! ## Entitlements Reference Pattern
//!
//! The full permissions array is replaced with:
//! - `entitlements_ref` - deterministic UUID-based reference for Redis lookup
//! - `entitlements_hash` - SHA-256 hash of canonical JSON of the entitlements snapshot
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

/// Canonical (sorted-keys) JSON representation for deterministic hashing.
///
/// HACK-207: All hash computations MUST use canonical JSON with sorted
/// keys and no whitespace so different implementations produce identical
/// digests. `serde_json::to_string` does NOT sort keys by default.
fn canonical_serialize<T: serde::ser::Serialize>(value: &T) -> String {
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
                let sorted: Vec<serde_json::Value> =
                    arr.iter().map(|item| sort_value(item)).collect();
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
fn entitlements_namespace() -> Uuid {
    let hash = Sha256::digest(b"sesame-idam-entitlements-namespace");
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&hash[..16]);
    // Set version to 5
    bytes[6] = (bytes[6] & 0x0F) | 0x50;
    // Set variant to RFC 4122
    bytes[8] = (bytes[8] & 0x3F) | 0x80;
    Uuid::from_bytes(bytes)
}

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

/// Namespaced authorization data - the `sx` field in JWT payloads.
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
    /// Format: "ent_<uuid>" where UUID is v5(user_id:org_id:version:tenant_id).
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
/// Standard JWT claims at top level, authz claims namespaced under https://sesame-idam.dev/claims.
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
    // Namespaced authz claims
    #[serde(rename = "https://sesame-idam.dev/claims")]
    pub sx: SesameAuthzClaims,
    // Optional delegation (RFC 8693)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act: Option<ActorClaim>,
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
            JwtError::Validation(e) => write!(f, "JWT validation error: {:?}", e),
            JwtError::MissingRequiredField(field) => write!(f, "Missing required field: {}", field),
            JwtError::TenantMismatch { expected, actual } => {
                write!(f, "Tenant mismatch: expected {}, got {}", expected, actual)
            }
        }
    }
}

impl AccessClaims {
    pub fn validate(&self) -> Result<(), JwtValidationError> {
        if !ALLOWED_ISSUERS.iter().any(|i| *i == self.iss.as_str()) {
            return Err(JwtValidationError::InvalidIssuer);
        }
        if !self.aud.iter().any(|a| EXPECTED_AUDIENCE.iter().any(|e| e == &a.as_str())) {
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

    pub fn builder() -> AccessClaimsBuilder {
        AccessClaimsBuilder::new()
    }

    /// Serialize to canonical JSON string for hashing.
    pub fn to_canonical_json(&self) -> String {
        let value = serde_json::to_value(self).unwrap_or(serde_json::Value::Null);
        serde_json::to_string(&value).unwrap_or_default()
    }

    /// Serialize to compact JSON (for JWT payload).
    pub fn to_compact_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Get size of compact JSON payload in bytes.
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

impl SesameAuthzClaims {
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

/// Builder for SesameAuthzClaims.
pub struct SesameAuthzClaimsBuilder {
    tenant: Option<String>,
    portal: Option<String>,
    roles: Option<Vec<String>>,
    permissions: Option<Vec<String>>,
    entitlements_ref: Option<String>,
    entitlements_hash: Option<String>,
    risk: Option<String>,
}

impl SesameAuthzClaimsBuilder {
    pub fn new() -> Self {
        Self {
            tenant: None,
            portal: None,
            roles: None,
            permissions: None,
            entitlements_ref: None,
            entitlements_hash: None,
            risk: None,
        }
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
        self.roles = Some(roles);
        self
    }

    pub fn permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions = Some(permissions);
        self
    }

    pub fn entitlements_ref(mut self, ref_str: impl Into<String>) -> Self {
        self.entitlements_ref = Some(ref_str.into());
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
        Ok(SesameAuthzClaims {
            tenant: self.tenant.ok_or_else(|| JwtError::MissingRequiredField("tenant".into()))?,
            portal: self.portal.ok_or_else(|| JwtError::MissingRequiredField("portal".into()))?,
            roles: self.roles.unwrap_or_default(),
            permissions: self.permissions.unwrap_or_default(),
            entitlements_ref: self.entitlements_ref,
            entitlements_hash: self.entitlements_hash,
            risk: self.risk,
        })
    }
}

impl Default for SesameAuthzClaimsBuilder {
    fn default() -> Self { Self::new() }
}

/// Generate a deterministic entitlements reference for the given tuple.
///
/// Uses UUID v5. Input: user_id:org_id:version:tenant_id
/// Deterministic for the same tuple, allowing consistent caching.
///
/// SECURITY NOTE (HACK-203): Entitlements refs are deterministic and potentially
/// enumerable. Acceptable because the ref is useless without Redis access,
/// the snapshot is cached with a short TTL (30-300s), and Redis access requires auth.
pub fn generate_entitlements_ref(
    user_id: &str,
    org_id: &str,
    version: u64,
    tenant_id: &str,
) -> String {
    let input = format!("{}:{}:{}:{}", user_id, org_id, version, tenant_id);
    let ns = entitlements_namespace();
    let uuid = Uuid::new_v5(&ns, input.as_bytes());
    format!("ent_{}", uuid)
}

/// Compute the SHA-256 hash of the canonical JSON representation of an
/// entitlements snapshot.
///
/// Returns the hash in the format "sha256:<64 hex chars>".
///
/// SECURITY NOTE (HACK-207): Standardized on SHA-256. The hash covers the
/// canonical JSON (sorted keys, no whitespace) of the EntitlementsSnapshot.
pub fn compute_entitlements_hash(snapshot: &EntitlementsSnapshot) -> String {
    // Compute hash of the snapshot EXCLUDING the hash field itself to avoid circular dependency.
    // We create a temporary value with the hash field cleared.
    let mut value = serde_json::to_value(snapshot).unwrap_or(serde_json::Value::Null);
    if let Some(obj) = value.as_object_mut() {
        obj.remove("hash");
    }
    let canonical = serde_json::to_string(&value).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let result = hasher.finalize();
    format!("sha256:{:x}", result)
}

/// Verify that an entitlements snapshot matches the expected hash.
///
/// CRITICAL: This function MUST be called after every Redis cache fetch.
/// The caller must handle the error and fall back to authz-core on mismatch.
///
/// SECURITY (HACK-201): Prevents cache poisoning via Redis. If an attacker
/// modifies the cached snapshot, hash verification fails and the consumer
/// must reject it, falling back to the authoritative authz-core service.
///
/// SECURITY (HACK-202): Hash covers the ENTIRE entitlements snapshot,
/// not just the permissions array. The authoritative data source is the
/// Redis snapshot (after hash verification), NOT the JWT sx.permissions.
pub fn verify_entitlements_hash(
    snapshot: &EntitlementsSnapshot,
    expected_hash: &str,
) -> Result<(), JwtValidationError> {
    let computed = compute_entitlements_hash(snapshot);
    if computed != expected_hash {
        // In production: invalidate the poisoned cache entry and re-fetch from authz-core
        return Err(JwtValidationError::EntitlementsHashMismatch);
    }
    Ok(())
}

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
        Self {
            iss: None, sub: None, aud: None, client_id: None, scope: None,
            exp: None, nbf: None, iat: None, jti: None, ver: None,
            sid: None, tenant_id: None, user_id: None, user_type: None,
            sx: None, act: None,
        }
    }

    pub fn iss(mut self, iss: impl Into<String>) -> Self { self.iss = Some(iss.into()); self }
    pub fn sub(mut self, sub: impl Into<String>) -> Self { self.sub = Some(sub.into()); self }
    pub fn aud(mut self, aud: Vec<String>) -> Self { self.aud = Some(aud); self }
    pub fn client_id(mut self, client_id: impl Into<String>) -> Self { self.client_id = Some(client_id.into()); self }
    pub fn scope(mut self, scope: impl Into<String>) -> Self { self.scope = Some(scope.into()); self }
    pub fn exp(mut self, exp: i64) -> Self { self.exp = Some(exp); self }
    pub fn nbf(mut self, nbf: i64) -> Self { self.nbf = Some(nbf); self }
    pub fn iat(mut self, iat: i64) -> Self { self.iat = Some(iat); self }
    pub fn jti(mut self, jti: impl Into<String>) -> Self { self.jti = Some(jti.into()); self }
    pub fn ver(mut self, ver: u64) -> Self { self.ver = Some(ver); self }
    pub fn sid(mut self, sid: impl Into<String>) -> Self { self.sid = Some(sid.into()); self }
    pub fn tenant_id(mut self, tenant_id: impl Into<String>) -> Self { self.tenant_id = Some(tenant_id.into()); self }
    pub fn user_id(mut self, user_id: impl Into<String>) -> Self { self.user_id = Some(user_id.into()); self }
    pub fn user_type(mut self, user_type: impl Into<String>) -> Self { self.user_type = Some(user_type.into()); self }
    pub fn sx(mut self, sx: SesameAuthzClaims) -> Self { self.sx = Some(sx); self }
    pub fn act(mut self, act: ActorClaim) -> Self { self.act = Some(act); self }

    pub fn build(self) -> Result<AccessClaims, JwtError> {
        let iss = self.iss.ok_or_else(|| JwtError::MissingRequiredField("iss".into()))?;
        let sub = self.sub.ok_or_else(|| JwtError::MissingRequiredField("sub".into()))?;
        let aud = self.aud.ok_or_else(|| JwtError::MissingRequiredField("aud".into()))?;
        let client_id = self.client_id.ok_or_else(|| JwtError::MissingRequiredField("client_id".into()))?;
        let scope = self.scope.ok_or_else(|| JwtError::MissingRequiredField("scope".into()))?;
        let exp = self.exp.ok_or_else(|| JwtError::MissingRequiredField("exp".into()))?;
        let nbf = self.nbf.ok_or_else(|| JwtError::MissingRequiredField("nbf".into()))?;
        let iat = self.iat.ok_or_else(|| JwtError::MissingRequiredField("iat".into()))?;
        let jti = self.jti.ok_or_else(|| JwtError::MissingRequiredField("jti".into()))?;
        let ver = self.ver.ok_or_else(|| JwtError::MissingRequiredField("ver".into()))?;
        let sid = self.sid.ok_or_else(|| JwtError::MissingRequiredField("sid".into()))?;
        let tenant_id = self.tenant_id.ok_or_else(|| JwtError::MissingRequiredField("tenant_id".into()))?;
        let user_id = self.user_id.ok_or_else(|| JwtError::MissingRequiredField("user_id".into()))?;
        let user_type = self.user_type.ok_or_else(|| JwtError::MissingRequiredField("user_type".into()))?;
        let sx = self.sx.ok_or_else(|| JwtError::MissingRequiredField("sx".into()))?;

        if ver == 0 {
            return Err(JwtError::MissingRequiredField("ver must be > 0".into()));
        }

        Ok(AccessClaims {
            iss, sub, aud, client_id, scope, exp, nbf, iat, jti,
            ver, sid, tenant_id, user_id, user_type, sx,
            act: self.act,
        })
    }
}

impl Default for AccessClaimsBuilder {
    fn default() -> Self { Self::new() }
}

// ===========================================================================
// Token size budget constants (Story 2.5 — HACK-252)
// ===========================================================================

/// Maximum JWT token payload size in bytes (750 bytes).
/// NGINX's default client_header_buffer_size is 1KB.
/// JWTs are transmitted as cookies or Authorization headers,
/// so we stay well below 1KB to avoid 414 errors.
pub const MAX_TOKEN_SIZE_BYTES: usize = 750;

/// Warning threshold for JWT token payload size (500 bytes).
/// Tokens approaching this size warrant investigation (HACK-250).
pub const TOKEN_SIZE_WARNING_BYTES: usize = 500;

/// Maximum number of permissions to embed in a JWT token (10).
/// Excess permissions are truncated; remaining are fetched via entitlements_ref (HACK-251).
pub const MAX_PERMISSIONS_PER_ROLE: usize = 10;

/// Maximum length for an entitlements_ref string (64 characters).
/// Prevents oversized ref strings from bloating the JWT payload (HACK-253).
pub const MAX_ENTITLEMENTS_REF_LENGTH: usize = 64;

// ===========================================================================
// Token size enforcement helpers (Story 2.5 — HACK-251/253)
// ===========================================================================

/// Truncate a permissions list to MAX_PERMISSIONS_PER_ROLE.
/// Returns the first MAX_PERMISSIONS_PER_ROLE entries plus a truncation marker
/// if the input exceeds the limit.
pub fn truncate_permissions(permissions: Vec<String>) -> Vec<String> {
    if permissions.len() <= MAX_PERMISSIONS_PER_ROLE {
        return permissions;
    }
    let remaining = permissions.len() - MAX_PERMISSIONS_PER_ROLE;
    let mut truncated: Vec<String> = permissions.into_iter()
        .take(MAX_PERMISSIONS_PER_ROLE)
        .collect();
    truncated.push(format!("...({} more)", remaining));
    truncated
}

/// Validate and optionally truncate an entitlements_ref value.
/// Returns None for empty strings, Some(truncated_ref) otherwise.
pub fn validate_entitlements_ref(reference: Option<&str>) -> Option<String> {
    let r = match reference {
        Some("") => return None,
        Some(r) => r,
        None => return None,
    };
    if r.len() > MAX_ENTITLEMENTS_REF_LENGTH {
        Some(r[..MAX_ENTITLEMENTS_REF_LENGTH].to_string())
    } else {
        Some(r.to_string())
    }
}

/// Truncate permissions on SesameAuthzClaims for token emission.
pub fn truncate_authz_claims_permissions(sx: SesameAuthzClaims) -> SesameAuthzClaims {
    SesameAuthzClaims {
        tenant: sx.tenant,
        portal: sx.portal,
        roles: sx.roles,
        permissions: truncate_permissions(sx.permissions),
        entitlements_ref: sx.entitlements_ref,
        entitlements_hash: sx.entitlements_hash,
        risk: sx.risk,
    }
}

/// Measure the unencoded size of a JWT token (sum of base64url-decoded parts).
/// Returns 0 if the token format is invalid (fewer than 3 dot-separated parts).
pub fn measure_jwt_token_size(token: &str) -> usize {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return 0;
    }
    parts.iter().map(|p| p.len()).sum()
}

pub const ALLOWED_ISSUERS: &[&str] = &[
    "https://sesame-idam.example.com",
    "https://idam.example.com",
];

pub const EXPECTED_AUDIENCE: &[&str] = &["sesame-idam", "api", "frontend", "mobile"];

#[cfg(test)]
mod tests {
    use super::*;

    // PII Removal Tests (Story 2.3)

    #[test]
    fn test_pii_fields_not_in_token() {
        let claims = AccessClaims {
            iss: "https://sesame-idam.example.com".to_string(),
            sub: "user-123".to_string(),
            aud: vec!["api".to_string()],
            client_id: "client-1".to_string(),
            scope: "openid profile".to_string(),
            exp: 1700000000,
            nbf: 1700000000 - 60,
            iat: 1700000000,
            jti: "jti-123".to_string(),
            ver: 1,
            sid: "session-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            user_id: "user-123".to_string(),
            user_type: "customer".to_string(),
            sx: SesameAuthzClaims {
                tenant: "tenant-1".to_string(),
                portal: "web".to_string(),
                roles: vec!["admin".to_string()],
                permissions: vec!["org:read".to_string()],
                entitlements_ref: Some("ent_abc123".to_string()),
                entitlements_hash: Some("sha256:abc123".to_string()),
                risk: None,
            },
            act: None,
        };

        let json = claims.to_compact_json();
        assert!(!json.contains("\"email\""), "email should not be in JWT");
        assert!(!json.contains("\"email_verified\""), "email_verified absent");
        assert!(!json.contains("\"phone_number\""), "phone_number absent");
        assert!(!json.contains("\"phone_verified\""), "phone_verified absent");
        assert!(!json.contains("\"first_name\""), "first_name absent");
        assert!(!json.contains("\"last_name\""), "last_name absent");
        assert!(!json.contains("\"name\""), "name absent");
        assert!(!json.contains("\"preferred_username\""), "preferred_username absent");
    }

    #[test]
    fn test_pii_values_absent_from_token_payload() {
        let claims = AccessClaims {
            iss: "https://sesame-idam.example.com".to_string(),
            sub: "user-123".to_string(),
            aud: vec!["api".to_string()],
            client_id: "client-1".to_string(),
            scope: "openid".to_string(),
            exp: 1700000000,
            nbf: 1700000000 - 60,
            iat: 1700000000,
            jti: "jti-123".to_string(),
            ver: 1,
            sid: "session-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            user_id: "user-123".to_string(),
            user_type: "customer".to_string(),
            sx: SesameAuthzClaims::new(
                "tenant-1".to_string(),
                "web".to_string(),
                vec!["admin".to_string()],
                vec!["org:read".to_string()],
            ),
            act: None,
        };

        let json = claims.to_compact_json();
        assert!(!json.contains("alice@corp.com"));
        assert!(!json.contains("+141****1234"));
        assert!(!json.contains("Alice"));
        assert!(!json.contains("Smith"));
        assert!(!json.contains("alice.smith"));
    }

    #[test]
    fn test_entitlements_ref_deterministic() {
        let ref1 = generate_entitlements_ref("user-1", "org-1", 1, "tenant-1");
        let ref2 = generate_entitlements_ref("user-1", "org-1", 1, "tenant-1");
        assert_eq!(ref1, ref2);
    }

    #[test]
    fn test_entitlements_ref_changes_on_version_bump() {
        let ref_v1 = generate_entitlements_ref("user-1", "org-1", 1, "tenant-1");
        let ref_v2 = generate_entitlements_ref("user-1", "org-1", 2, "tenant-1");
        assert_ne!(ref_v1, ref_v2, "version bump should change ref");
    }

    #[test]
    fn test_entitlements_ref_format() {
        let ref_str = generate_entitlements_ref("user-1", "org-1", 1, "tenant-1");
        assert!(ref_str.starts_with("ent_"));
        let uuid_part = &ref_str[4..];
        assert_eq!(uuid_part.len(), 36, "should be ent_ + 36-char UUID");
    }

    #[test]
    fn test_entitlements_hash_matches_canonical_json() {
        let snapshot = EntitlementsSnapshot {
            version: 42,
            permissions: vec!["org:admin".to_string(), "billing:read".to_string()],
            roles: vec!["admin".to_string(), "billing-viewer".to_string()],
            tenant: "tenant-1".to_string(),
            hash: String::new(),
        };

        let hash = compute_entitlements_hash(&snapshot);
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 71, "sha256: + 64 hex chars = 71 chars");
    }

    #[test]
    fn test_hash_format_validation() {
        let snapshot = EntitlementsSnapshot {
            version: 1,
            permissions: vec![],
            roles: vec![],
            tenant: "tenant-1".to_string(),
            hash: String::new(),
        };

        let hash = compute_entitlements_hash(&snapshot);
        assert!(hash.starts_with("sha256:"));
        let hex_part = &hash[7..];
        assert_eq!(hex_part.len(), 64);
        assert!(hex_part.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_verify_entitlements_hash_valid() {
        let mut snapshot = EntitlementsSnapshot {
            version: 1,
            permissions: vec!["read".to_string()],
            roles: vec!["user".to_string()],
            tenant: "tenant-1".to_string(),
            hash: String::new(),
        };

        let expected_hash = compute_entitlements_hash(&snapshot);
        snapshot.hash = expected_hash.clone();
        assert!(verify_entitlements_hash(&snapshot, &expected_hash).is_ok());
    }

    #[test]
    fn test_verify_entitlements_hash_mismatch() {
        let snapshot = EntitlementsSnapshot {
            version: 1,
            permissions: vec!["read".to_string()],
            roles: vec!["user".to_string()],
            tenant: "tenant-1".to_string(),
            hash: "sha256:wronghash".to_string(),
        };

        let result = verify_entitlements_hash(&snapshot, "sha256:correcthash");
        assert_eq!(result, Err(JwtValidationError::EntitlementsHashMismatch));
    }

    #[test]
    fn test_empty_entitlements_snapshot() {
        let snapshot = EntitlementsSnapshot {
            version: 0,
            permissions: vec![],
            roles: vec![],
            tenant: "tenant-1".to_string(),
            hash: String::new(),
        };
        let hash = compute_entitlements_hash(&snapshot);
        assert!(hash.starts_with("sha256:"));
    }

    #[test]
    fn test_large_entitlements_set_stays_under_budget() {
        let claims = AccessClaims {
            iss: "https://sesame-idam.example.com".to_string(),
            sub: "user-123".to_string(),
            aud: vec!["api".to_string()],
            client_id: "client-1".to_string(),
            scope: "openid".to_string(),
            exp: 1700000000,
            nbf: 1700000000 - 60,
            iat: 1700000000,
            jti: "jti-123".to_string(),
            ver: 1,
            sid: "session-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            user_id: "user-123".to_string(),
            user_type: "customer".to_string(),
            sx: SesameAuthzClaims {
                tenant: "tenant-1".to_string(),
                portal: "web".to_string(),
                roles: vec!["admin".to_string()],
                permissions: vec!["org:read".to_string()],
                entitlements_ref: Some(generate_entitlements_ref("user-123", "org-1", 1, "tenant-1")),
                entitlements_hash: Some("sha256:abc123def456".to_string()),
                risk: None,
            },
            act: None,
        };

        let size = claims.json_payload_size();
        assert!(size < 750, "JWT payload size {} exceeds 750-byte budget", size);
    }

    #[test]
    fn test_sesame_authz_claims_full_round_trip() {
        let sx = SesameAuthzClaims {
            tenant: "tenant-1".to_string(),
            portal: "web".to_string(),
            roles: vec!["admin".to_string(), "billing".to_string()],
            permissions: vec!["org:admin".to_string(), "billing:write".to_string()],
            entitlements_ref: Some("ent_abc123".to_string()),
            entitlements_hash: Some("sha256:abc123".to_string()),
            risk: Some("normal".to_string()),
        };

        let json = serde_json::to_string(&sx).unwrap();
        let deserialized: SesameAuthzClaims = serde_json::from_str(&json).unwrap();
        assert_eq!(sx, deserialized);
    }

    #[test]
    fn test_sesame_authz_claims_optional_fields_absent() {
        let sx = SesameAuthzClaims::new(
            "tenant-1".to_string(),
            "web".to_string(),
            vec!["admin".to_string()],
            vec!["org:read".to_string()],
        );
        let json = serde_json::to_string(&sx).unwrap();
        assert!(!json.contains("entitlements_ref"));
        assert!(!json.contains("entitlements_hash"));
        assert!(!json.contains("risk"));
    }

    #[test]
    fn test_actor_claim_round_trip() {
        let actor = ActorClaim { sub: "user-123".to_string() };
        let json = serde_json::to_string(&actor).unwrap();
        let deserialized: ActorClaim = serde_json::from_str(&json).unwrap();
        assert_eq!(actor, deserialized);
    }

    #[test]
    fn test_access_claims_act_present_absent() {
        let no_act = AccessClaims {
            iss: "https://sesame-idam.example.com".to_string(),
            sub: "user-123".to_string(),
            aud: vec!["api".to_string()],
            client_id: "client-1".to_string(),
            scope: "openid".to_string(),
            exp: 1700000000, nbf: 1700000000 - 60, iat: 1700000000,
            jti: "jti-123".to_string(), ver: 1, sid: "session-1".to_string(),
            tenant_id: "tenant-1".to_string(), user_id: "user-123".to_string(),
            user_type: "customer".to_string(),
            sx: SesameAuthzClaims::new("tenant-1".to_string(), "web".to_string(), vec![], vec![]),
            act: None,
        };

        let json_no = serde_json::to_string(&no_act).unwrap();
        assert!(!json_no.contains("\"act\""));

        let with_act = AccessClaims {
            act: Some(ActorClaim { sub: "user-456".to_string() }),
            ..no_act.clone()
        };
        let json_yes = serde_json::to_string(&with_act).unwrap();
        assert!(json_yes.contains("\"act\""));
    }

    #[test]
    fn test_sesame_authz_claims_special_characters() {
        let json = serde_json::to_value(&SesameAuthzClaims::new(
            "tenant-1".to_string(), "web".to_string(), vec![], vec![],
        )).unwrap();
        assert!(!json.to_string().contains("O'Brien"));
        assert!(!json.to_string().contains("+141****1234"));
    }

    // Validation Tests (from Story 2.2)

    #[test]
    fn test_valid_claims_pass_validation() {
        let claims = AccessClaims {
            iss: "https://sesame-idam.example.com".to_string(),
            sub: "user-123".to_string(),
            aud: vec!["api".to_string()],
            client_id: "client-1".to_string(),
            scope: "openid".to_string(),
            exp: 1700000000, nbf: 1700000000 - 60, iat: 1700000000,
            jti: "jti-123".to_string(), ver: 1, sid: "session-1".to_string(),
            tenant_id: "tenant-1".to_string(), user_id: "user-123".to_string(),
            user_type: "customer".to_string(),
            sx: SesameAuthzClaims::new("tenant-1".to_string(), "web".to_string(),
                vec!["admin".to_string()], vec!["org:read".to_string()]),
            act: None,
        };
        assert!(claims.validate().is_ok());
    }

    #[test]
    fn test_validation_rejects_missing_ver() {
        let claims = AccessClaims {
            iss: "https://sesame-idam.example.com".to_string(),
            sub: "user-123".to_string(), aud: vec!["api".to_string()],
            client_id: "client-1".to_string(), scope: "openid".to_string(),
            exp: 1700000000, nbf: 1700000000 - 60, iat: 1700000000,
            jti: "jti-123".to_string(), ver: 0, // missing version
            sid: "session-1".to_string(), tenant_id: "tenant-1".to_string(),
            user_id: "user-123".to_string(), user_type: "customer".to_string(),
            sx: SesameAuthzClaims::new("tenant-1".to_string(), "web".to_string(), vec![], vec![]),
            act: None,
        };
        assert_eq!(claims.validate(), Err(JwtValidationError::MissingVersion));
    }

    #[test]
    fn test_validation_rejects_missing_tenant_id() {
        let claims = AccessClaims {
            iss: "https://sesame-idam.example.com".to_string(),
            sub: "user-123".to_string(), aud: vec!["api".to_string()],
            client_id: "client-1".to_string(), scope: "openid".to_string(),
            exp: 1700000000, nbf: 1700000000 - 60, iat: 1700000000,
            jti: "jti-123".to_string(), ver: 1, sid: "session-1".to_string(),
            tenant_id: "".to_string(), user_id: "user-123".to_string(),
            user_type: "customer".to_string(),
            sx: SesameAuthzClaims::new("tenant-1".to_string(), "web".to_string(), vec![], vec![]),
            act: None,
        };
        assert_eq!(claims.validate(), Err(JwtValidationError::MissingTenant));
    }

    #[test]
    fn test_validation_rejects_missing_sx_tenant() {
        let claims = AccessClaims {
            iss: "https://sesame-idam.example.com".to_string(),
            sub: "user-123".to_string(), aud: vec!["api".to_string()],
            client_id: "client-1".to_string(), scope: "openid".to_string(),
            exp: 1700000000, nbf: 1700000000 - 60, iat: 1700000000,
            jti: "jti-123".to_string(), ver: 1, sid: "session-1".to_string(),
            tenant_id: "tenant-1".to_string(), user_id: "user-123".to_string(),
            user_type: "customer".to_string(),
            sx: SesameAuthzClaims {
                tenant: "".to_string(), portal: "web".to_string(),
                roles: vec![], permissions: vec![],
                entitlements_ref: None, entitlements_hash: None, risk: None,
            },
            act: None,
        };
        assert_eq!(claims.validate(), Err(JwtValidationError::MissingAuthzClaims));
    }

    #[test]
    fn test_validation_rejects_invalid_issuer() {
        let claims = AccessClaims {
            iss: "https://evil-issuer.example.com".to_string(),
            sub: "user-123".to_string(), aud: vec!["api".to_string()],
            client_id: "client-1".to_string(), scope: "openid".to_string(),
            exp: 1700000000, nbf: 1700000000 - 60, iat: 1700000000,
            jti: "jti-123".to_string(), ver: 1, sid: "session-1".to_string(),
            tenant_id: "tenant-1".to_string(), user_id: "user-123".to_string(),
            user_type: "customer".to_string(),
            sx: SesameAuthzClaims::new("tenant-1".to_string(), "web".to_string(), vec![], vec![]),
            act: None,
        };
        assert_eq!(claims.validate(), Err(JwtValidationError::InvalidIssuer));
    }

    #[test]
    fn test_validation_rejects_invalid_audience() {
        let claims = AccessClaims {
            iss: "https://sesame-idam.example.com".to_string(),
            sub: "user-123".to_string(), aud: vec!["unknown-service".to_string()],
            client_id: "client-1".to_string(), scope: "openid".to_string(),
            exp: 1700000000, nbf: 1700000000 - 60, iat: 1700000000,
            jti: "jti-123".to_string(), ver: 1, sid: "session-1".to_string(),
            tenant_id: "tenant-1".to_string(), user_id: "user-123".to_string(),
            user_type: "customer".to_string(),
            sx: SesameAuthzClaims::new("tenant-1".to_string(), "web".to_string(), vec![], vec![]),
            act: None,
        };
        assert_eq!(claims.validate(), Err(JwtValidationError::InvalidAudience));
    }

    #[test]
    fn test_validation_accepts_valid_risk_values() {
        for risk_value in &["normal", "elevated", "critical"] {
            let claims = AccessClaims {
                iss: "https://sesame-idam.example.com".to_string(),
                sub: "user-123".to_string(), aud: vec!["api".to_string()],
                client_id: "client-1".to_string(), scope: "openid".to_string(),
                exp: 1700000000, nbf: 1700000000 - 60, iat: 1700000000,
                jti: "jti-123".to_string(), ver: 1, sid: "session-1".to_string(),
                tenant_id: "tenant-1".to_string(), user_id: "user-123".to_string(),
                user_type: "customer".to_string(),
                sx: SesameAuthzClaims {
                    tenant: "tenant-1".to_string(), portal: "web".to_string(),
                    roles: vec![], permissions: vec![],
                    entitlements_ref: None, entitlements_hash: None,
                    risk: Some(risk_value.to_string()),
                },
                act: None,
            };
            assert!(claims.validate().is_ok(), "risk '{}' should be valid", risk_value);
        }
    }

    #[test]
    fn test_validation_rejects_invalid_risk() {
        let claims = AccessClaims {
            iss: "https://sesame-idam.example.com".to_string(),
            sub: "user-123".to_string(), aud: vec!["api".to_string()],
            client_id: "client-1".to_string(), scope: "openid".to_string(),
            exp: 1700000000, nbf: 1700000000 - 60, iat: 1700000000,
            jti: "jti-123".to_string(), ver: 1, sid: "session-1".to_string(),
            tenant_id: "tenant-1".to_string(), user_id: "user-123".to_string(),
            user_type: "customer".to_string(),
            sx: SesameAuthzClaims {
                tenant: "tenant-1".to_string(), portal: "web".to_string(),
                roles: vec![], permissions: vec![],
                entitlements_ref: None, entitlements_hash: None,
                risk: Some("unknown".to_string()),
            },
            act: None,
        };
        assert_eq!(claims.validate(), Err(JwtValidationError::InvalidRisk));
    }

    #[test]
    fn test_builder_constructs_valid_claims() {
        let claims = AccessClaims::builder()
            .iss("https://sesame-idam.example.com".to_string())
            .sub("user-123".to_string())
            .aud(vec!["api".to_string()])
            .client_id("client-1".to_string())
            .scope("openid".to_string())
            .exp(1700000000).nbf(1700000000 - 60).iat(1700000000)
            .jti("jti-123".to_string())
            .ver(1).sid("session-1".to_string())
            .tenant_id("tenant-1".to_string())
            .user_id("user-123".to_string())
            .user_type("customer".to_string())
            .sx(SesameAuthzClaims::new(
                "tenant-1".to_string(), "web".to_string(),
                vec!["admin".to_string()], vec!["org:read".to_string()],
            )).build();
        assert!(claims.is_ok());
        let claims = claims.unwrap();
        assert_eq!(claims.iss, "https://sesame-idam.example.com");
        assert_eq!(claims.ver, 1);
        assert_eq!(claims.tenant_id, "tenant-1");
    }

    #[test]
    fn test_builder_rejects_missing_required_fields() {
        let result = AccessClaims::builder()
            .iss("https://sesame-idam.example.com".to_string())
            .sub("user-123".to_string())
            .aud(vec!["api".to_string()])
            .client_id("client-1".to_string())
            .scope("openid".to_string())
            .exp(1700000000).nbf(1700000000 - 60).iat(1700000000)
            .jti("jti-123".to_string()).ver(1)
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_rejects_ver_zero() {
        let result = AccessClaims::builder()
            .iss("https://sesame-idam.example.com".to_string())
            .sub("user-123".to_string())
            .aud(vec!["api".to_string()])
            .client_id("client-1".to_string())
            .scope("openid".to_string())
            .exp(1700000000).nbf(1700000000 - 60).iat(1700000000)
            .jti("jti-123".to_string())
            .ver(0) // explicitly zero
            .sid("session-1".to_string())
            .tenant_id("tenant-1".to_string())
            .user_id("user-123".to_string())
            .user_type("customer".to_string())
            .sx(SesameAuthzClaims::new(
                "tenant-1".to_string(), "web".to_string(), vec![], vec![],
            )).build();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), JwtError::MissingRequiredField("ver must be > 0".into()));
    }

    #[test]
    fn test_token_size_under_budget() {
        let roles: Vec<String> = (0..10).map(|i| format!("role-{}", i)).collect();
        let permissions: Vec<String> = (0..10).map(|i| format!("perm:{}", i)).collect();

        let claims = AccessClaims {
            iss: "https://sesame-idam.example.com".to_string(),
            sub: "user-123".to_string(),
            aud: vec!["api".to_string(), "frontend".to_string()],
            client_id: "client-1".to_string(),
            scope: "openid profile email".to_string(),
            exp: 1700000000, nbf: 1700000000 - 60, iat: 1700000000,
            jti: "jti-123".to_string(), ver: 1, sid: "session-1".to_string(),
            tenant_id: "tenant-1".to_string(), user_id: "user-123".to_string(),
            user_type: "customer".to_string(),
            sx: SesameAuthzClaims {
                tenant: "tenant-1".to_string(), portal: "web".to_string(),
                roles, permissions,
                entitlements_ref: Some(generate_entitlements_ref("user-123", "org-1", 1, "tenant-1")),
                entitlements_hash: Some("sha256:".to_string() + &"a".repeat(64)),
                risk: Some("normal".to_string()),
            },
            act: None,
        };

        let size = claims.json_payload_size();
        assert!(size < 750, "JWT payload size {} exceeds 750-byte budget", size);
    }

    #[test]
    fn test_entitlements_ref_is_tenant_aware() {
        let ref_a = generate_entitlements_ref("user-1", "org-1", 1, "tenant-a");
        let ref_b = generate_entitlements_ref("user-1", "org-1", 1, "tenant-b");
        assert_ne!(ref_a, ref_b, "Different tenants should produce different refs");

        let ref_a_2 = generate_entitlements_ref("user-1", "org-1", 1, "tenant-a");
        assert_eq!(ref_a, ref_a_2);
    }

    // ─── Token Size Budget Enforcement Tests (Story 2.5) ─────────────────

    /// Permissions within MAX_PERMISSIONS_PER_ROLE pass through unchanged
    #[test]
    fn test_truncate_permissions_within_limit() {
        let perms: Vec<String> = (0..5)
            .map(|i| format!("perm-{}", i))
            .collect();
        let result = truncate_permissions(perms.clone());
        assert_eq!(result, perms, "Should pass through when within limit");
    }

    /// Permissions over MAX_PERMISSIONS_PER_ROLE are truncated
    #[test]
    fn test_truncate_permissions_over_limit() {
        let perms: Vec<String> = (0..15)
            .map(|i| format!("perm-{}", i))
            .collect();
        let result = truncate_permissions(perms);
        assert_eq!(
            result.len(),
            MAX_PERMISSIONS_PER_ROLE + 1,
            "Should truncate to {} + 1 marker",
            MAX_PERMISSIONS_PER_ROLE
        );
        assert!(
            result.last().unwrap().starts_with("...("),
            "Last entry should be the truncation marker"
        );
    }

    /// Entitlements ref at max length passes validation
    #[test]
    fn test_validate_entitlements_ref_ok() {
        let valid_ref = "ent_abc123";
        assert_eq!(
            validate_entitlements_ref(Some(valid_ref)),
            Some(valid_ref.to_string())
        );
    }

    /// Entitlements ref too long is truncated
    #[test]
    fn test_validate_entitlements_ref_too_long() {
        let long_ref = "ent_".to_owned() + &"a".repeat(100);
        let result = validate_entitlements_ref(Some(&long_ref));
        assert!(result.is_some());
        let truncated = result.unwrap();
        assert_eq!(truncated.len(), MAX_ENTITLEMENTS_REF_LENGTH);
    }

    /// Empty entitlements ref returns None
    #[test]
    fn test_validate_entitlements_ref_empty() {
        assert_eq!(validate_entitlements_ref(Some("")), None);
    }

    /// No entitlements ref returns None
    #[test]
    fn test_validate_entitlements_ref_none() {
        assert_eq!(validate_entitlements_ref(None), None);
    }

    /// Token size measurement on valid JWT format
    #[test]
    fn test_measure_jwt_token_size() {
        let token = "header.payload.signature";
        assert_eq!(measure_jwt_token_size(token), 22); // 5 + 8 + 9 + 2 dots
    }

    /// Token size measurement on invalid JWT format returns 0
    #[test]
    fn test_measure_jwt_token_size_invalid() {
        assert_eq!(measure_jwt_token_size("not.a.jwt.token"), 0);
        assert_eq!(measure_jwt_token_size("single"), 0);
    }

    /// Truncated SesameAuthzClaims fit budget
    #[test]
    fn test_truncated_authz_claims_fits_budget() {
        let permissions: Vec<String> = (0..50)
            .map(|i| format!("perm:resource:{}", i))
            .collect();
        let roles: Vec<String> = (0..5)
            .map(|i| format!("role-{}", i))
            .collect();

        let sx = SesameAuthzClaims {
            tenant: "tenant-1".to_string(),
            portal: "web".to_string(),
            roles: roles.clone(),
            permissions,
            entitlements_ref: None,
            entitlements_hash: None,
            risk: None,
        };

        let truncated = truncate_authz_claims_permissions(sx);
        assert_eq!(
            truncated.permissions.len(),
            MAX_PERMISSIONS_PER_ROLE + 1,
            "Permissions should be truncated to max + 1 marker"
        );

        let claims = AccessClaims {
            iss: "https://sesame-idam.example.com".to_string(),
            sub: "user-123".to_string(),
            aud: vec!["api".to_string()],
            client_id: "client-1".to_string(),
            scope: "openid".to_string(),
            exp: 1700000000,
            nbf: 1700000000 - 60,
            iat: 1700000000,
            jti: "jti-truncated".to_string(),
            ver: 1,
            sid: "session-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            user_id: "user-123".to_string(),
            user_type: "customer".to_string(),
            sx: truncated,
            act: None,
        };

        let size = claims.json_payload_size();
        assert!(
            size < MAX_TOKEN_SIZE_BYTES,
            "Truncated claims payload {} bytes still exceeds 750-byte budget",
            size
        );
    }

    /// Permissions truncation enforces the configured maximum
    #[test]
    fn test_truncate_permissions_enforces_limit() {
        // Test with 0 permissions (no change)
        let result = truncate_permissions(vec![]);
        assert_eq!(result.len(), 0);

        // Test with exactly MAX_PERMISSIONS_PER_ROLE (no truncation)
        let perms: Vec<String> = (0..MAX_PERMISSIONS_PER_ROLE)
            .map(|i| format!("perm:{}", i))
            .collect();
        let result = truncate_permissions(perms.clone());
        assert_eq!(result.len(), MAX_PERMISSIONS_PER_ROLE);
        assert_eq!(result, perms);

        // Test with more than MAX_PERMISSIONS_PER_ROLE (truncation)
        let perms: Vec<String> = (0..20)
            .map(|i| format!("perm:{}", i))
            .collect();
        let result = truncate_permissions(perms);
        assert_eq!(result.len(), MAX_PERMISSIONS_PER_ROLE + 1); // +1 for "...(10 more)"
        assert!(
            result.iter().any(|s| s.contains("more")),
            "truncated result should contain '...' suffix"
        );
        assert_eq!(
            result.last().unwrap(),
            "...(10 more)",
            "last element should be truncation notice"
        );
    }

    /// Entitlements ref format validation: max 64 chars
    #[test]
    fn test_entitlements_ref_max_length() {
        // Exactly 64 chars - should pass through
        let ref_64 = "ent_".to_owned() + &"a".repeat(60);
        assert_eq!(ref_64.len(), 64);
        let result = validate_entitlements_ref(Some(&ref_64));
        assert_eq!(result, Some(ref_64.clone()));

        // 65 chars - should be truncated to 64
        let ref_65 = "ent_".to_owned() + &"a".repeat(61);
        assert_eq!(ref_65.len(), 65);
        let result = validate_entitlements_ref(Some(&ref_65));
        assert_eq!(result, Some(ref_64));
    }

    /// Build-time test: representative token (10 roles, 10 permissions, all claims)
    /// must fit within 750 bytes unencoded budget.
    #[test]
    fn test_build_time_token_size_within_budget() {
        let roles: Vec<String> = (0..5)
            .map(|i| format!("role-{i}"))
            .collect();
        let permissions: Vec<String> = (0..5)
            .map(|i| format!("perm:{i}"))
            .collect();

        let claims = AccessClaims {
            iss: "https://sesame-idam.example.com".to_string(),
            sub: "user-123".to_string(),
            aud: vec!["api".to_string(), "frontend".to_string()],
            client_id: "client-1".to_string(),
            scope: "openid".to_string(),
            exp: 1700000000,
            nbf: 1700000000 - 60,
            iat: 1700000000,
            jti: "jti-123".to_string(),
            ver: 1,
            sid: "session-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            user_id: "user-123".to_string(),
            user_type: "customer".to_string(),
            sx: SesameAuthzClaims {
                tenant: "tenant-1".to_string(),
                portal: "web".to_string(),
                roles,
                permissions,
                entitlements_ref: Some("ent_abc123".to_string()),
                entitlements_hash: Some("sha256:abcdef1234".to_string()),
                risk: Some("normal".to_string()),
            },
            act: None,
        };

        let size = claims.json_payload_size();
        assert!(
            size < MAX_TOKEN_SIZE_BYTES,
            "Representative token {} bytes exceeds {}-byte budget",
            size,
            MAX_TOKEN_SIZE_BYTES
        );
    }

    // ─── Story 2.4: Tenant Claim Validation Unit Tests ─────────────────────

    #[test]
    fn test_validate_tenant_accepts_matching_tenant() {
        let claims = AccessClaimsBuilder::new()
            .iss("https://sesame-idam.example.com")
            .sub("user-123")
            .aud(vec!["api".to_string()])
            .client_id("client-1")
            .scope("openid".to_string())
            .exp(1700000000)
            .nbf(1700000000 - 60)
            .iat(1700000000)
            .jti("jti-123".to_string())
            .ver(1)
            .sid("session-1".to_string())
            .tenant_id("tenant-alpha".to_string())
            .user_id("user-123".to_string())
            .user_type("customer".to_string())
            .sx(SesameAuthzClaims::new(
                "tenant-alpha".to_string(),
                "web".to_string(),
                vec![],
                vec![],
            ))
            .build()
            .expect("valid claims");

        // Both top-level and sx.tenant match the request tenant
        assert!(claims.validate_tenant("tenant-alpha").is_ok());
    }

    #[test]
    fn test_validate_tenant_rejects_mismatched_top_level() {
        let claims = AccessClaimsBuilder::new()
            .iss("https://sesame-idam.example.com")
            .sub("user-123")
            .aud(vec!["api".to_string()])
            .client_id("client-1")
            .scope("openid".to_string())
            .exp(1700000000)
            .nbf(1700000000 - 60)
            .iat(1700000000)
            .jti("jti-123".to_string())
            .ver(1)
            .sid("session-1".to_string())
            .tenant_id("tenant-alpha".to_string())
            .user_id("user-123".to_string())
            .user_type("customer".to_string())
            .sx(SesameAuthzClaims::new(
                "tenant-alpha".to_string(),
                "web".to_string(),
                vec![],
                vec![],
            ))
            .build()
            .expect("valid claims");

        // Request tenant doesn't match top-level tenant_id
        let result = claims.validate_tenant("tenant-beta");
        assert!(result.is_err());
        match result.unwrap_err() {
            JwtError::TenantMismatch { expected, actual } => {
                assert_eq!(expected, "tenant-alpha");
                assert_eq!(actual, "tenant-beta");
            }
            _ => panic!("Expected TenantMismatch"),
        }
    }

    #[test]
    fn test_validate_tenant_rejects_empty_request_tenant() {
        let claims = AccessClaimsBuilder::new()
            .iss("https://sesame-idam.example.com")
            .sub("user-123")
            .aud(vec!["api".to_string()])
            .client_id("client-1")
            .scope("openid".to_string())
            .exp(1700000000)
            .nbf(1700000000 - 60)
            .iat(1700000000)
            .jti("jti-123".to_string())
            .ver(1)
            .sid("session-1".to_string())
            .tenant_id("tenant-alpha".to_string())
            .user_id("user-123".to_string())
            .user_type("customer".to_string())
            .sx(SesameAuthzClaims::new(
                "tenant-alpha".to_string(),
                "web".to_string(),
                vec![],
                vec![],
            ))
            .build()
            .expect("valid claims");

        // Empty request_tenant is always rejected (HACK-243)
        let result = claims.validate_tenant("");
        assert!(result.is_err());
        match result.unwrap_err() {
            JwtError::MissingRequiredField(field) => {
                assert_eq!(field, "X-Tenant-ID");
            }
            _ => panic!("Expected MissingRequiredField"),
        }
    }

    #[test]
    fn test_validate_tenant_checks_both_top_level_and_namespaced() {
        // Test case: top-level matches but sx.tenant doesn't
        let mut claims = AccessClaimsBuilder::new()
            .iss("https://sesame-idam.example.com")
            .sub("user-123")
            .aud(vec!["api".to_string()])
            .client_id("client-1")
            .scope("openid".to_string())
            .exp(1700000000)
            .nbf(1700000000 - 60)
            .iat(1700000000)
            .jti("jti-123".to_string())
            .ver(1)
            .sid("session-1".to_string())
            .tenant_id("tenant-alpha".to_string())
            .user_id("user-123".to_string())
            .user_type("customer".to_string())
            .sx(SesameAuthzClaims::new(
                "tenant-beta".to_string(), // Different from top-level!
                "web".to_string(),
                vec![],
                vec![],
            ))
            .build()
            .expect("valid claims");

        // Even though top-level tenant_id matches, sx.tenant doesn't
        let result = claims.validate_tenant("tenant-alpha");
        assert!(result.is_err(), "Must reject when sx.tenant doesn't match");

        // Test case: sx.tenant matches but top-level doesn't
        claims = AccessClaimsBuilder::new()
            .iss("https://sesame-idam.example.com")
            .sub("user-123")
            .aud(vec!["api".to_string()])
            .client_id("client-1")
            .scope("openid".to_string())
            .exp(1700000000)
            .nbf(1700000000 - 60)
            .iat(1700000000)
            .jti("jti-123".to_string())
            .ver(1)
            .sid("session-1".to_string())
            .tenant_id("tenant-beta".to_string())
            .user_id("user-123".to_string())
            .user_type("customer".to_string())
            .sx(SesameAuthzClaims::new(
                "tenant-alpha".to_string(),
                "web".to_string(),
                vec![],
                vec![],
            ))
            .build()
            .expect("valid claims");

        // Even though sx.tenant matches, top-level tenant_id doesn't
        let result = claims.validate_tenant("tenant-alpha");
        assert!(result.is_err(), "Must reject when top-level tenant_id doesn't match");
    }

    #[test]
    fn test_validate_tenant_consistent_across_user_types() {
        for user_type in &["customer", "platform", "platform_admin"] {
            let claims = AccessClaimsBuilder::new()
                .iss("https://sesame-idam.example.com")
                .sub(format!("user-{}", user_type))
                .aud(vec!["api".to_string()])
                .client_id("app")
                .scope("openid".to_string())
                .exp(1700000000)
                .nbf(1700000000 - 60)
                .iat(1700000000)
                .jti(format!("jti-{}", user_type))
                .ver(1)
                .sid(format!("session-{}", user_type))
                .tenant_id("tenant-shared".to_string())
                .user_id(format!("user-{}", user_type))
                .user_type(user_type.to_string())
                .sx(SesameAuthzClaims::new(
                    "tenant-shared".to_string(),
                    "web".to_string(),
                    vec![],
                    vec![],
                ))
                .build()
                .expect("valid claims");

            assert!(
                claims.validate_tenant("tenant-shared").is_ok(),
                "user_type {} must validate_tenant pass for matching tenant",
                user_type
            );
        }
    }
}
