//! JWT helper functions: entitlements refs/hashes, size enforcement, constants.

use sha2::{Digest, Sha256};
use uuid::Uuid;

// ===========================================================================
// Type re-exports needed by helpers
// ===========================================================================

pub use super::types::{AccessClaims, EntitlementsSnapshot, JwtValidationError, SesameAuthzClaims};

// ===========================================================================
// Token size budget constants (Story 2.5 — HACK-252)
// ===========================================================================

/// Maximum JWT token payload size in bytes (750 bytes).
/// NGINX's default `client_header_buffer_size` is 1KB.
/// JWTs are transmitted as cookies or Authorization headers,
/// so we stay well below 1KB to avoid 414 errors.
pub const MAX_TOKEN_SIZE_BYTES: usize = 750;

/// Warning threshold for JWT token payload size (500 bytes).
/// Tokens approaching this size warrant investigation (HACK-250).
pub const TOKEN_SIZE_WARNING_BYTES: usize = 500;

/// Maximum number of permissions to embed in a JWT token (10).
/// Excess permissions are truncated; remaining are fetched via `entitlements_ref` (HACK-251).
pub const MAX_PERMISSIONS_PER_ROLE: usize = 10;

/// Maximum length for an `entitlements_ref` string (64 characters).
/// Prevents oversized ref strings from bloating the JWT payload (HACK-253).
pub const MAX_ENTITLEMENTS_REF_LENGTH: usize = 64;

// ===========================================================================
// Token size enforcement helpers (Story 2.5 — HACK-251/253)
// ===========================================================================

/// Truncate a permissions list to `MAX_PERMISSIONS_PER_ROLE`.
/// Returns the first `MAX_PERMISSIONS_PER_ROLE` entries plus a truncation marker
/// if the input exceeds the limit.
#[must_use]
pub fn truncate_permissions(permissions: Vec<String>) -> Vec<String> {
    if permissions.len() <= MAX_PERMISSIONS_PER_ROLE {
        return permissions;
    }
    let remaining = permissions.len() - MAX_PERMISSIONS_PER_ROLE;
    let mut truncated: Vec<String> = permissions
        .into_iter()
        .take(MAX_PERMISSIONS_PER_ROLE)
        .collect();
    truncated.push(format!("...({remaining} more)"));
    truncated
}

/// Validate and optionally truncate an `entitlements_ref` value.
/// Returns None for empty strings, `Some(truncated_ref)` otherwise.
#[must_use]
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

/// Truncate permissions on `SesameAuthzClaims` for token emission.
#[must_use]
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
#[must_use]
pub fn measure_jwt_token_size(token: &str) -> usize {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return 0;
    }
    parts.iter().map(|p| p.len()).sum()
}

// ===========================================================================
// Entitlements ref and hash functions
// ===========================================================================

/// Generate a deterministic entitlements reference for the given tuple.
///
/// Uses UUID v5. Input: `user_id:org_id:version:tenant_id`
/// Deterministic for the same tuple, allowing consistent caching.
///
/// SECURITY NOTE (HACK-203): Entitlements refs are deterministic and potentially
/// enumerable. Acceptable because the ref is useless without Redis access,
/// the snapshot is cached with a short TTL (30-300s), and Redis access requires auth.
#[must_use]
pub fn generate_entitlements_ref(
    user_id: &str,
    org_id: &str,
    version: u64,
    tenant_id: &str,
) -> String {
    let input = format!("{user_id}:{org_id}:{version}:{tenant_id}");
    let ns = super::types::entitlements_namespace();
    let uuid = Uuid::new_v5(&ns, input.as_bytes());
    format!("ent_{uuid}")
}

/// Compute the SHA-256 hash of the canonical JSON representation of an
/// entitlements snapshot.
///
/// Returns the hash in the format "sha256:<64 hex chars>".
///
/// SECURITY NOTE (HACK-207): Standardized on SHA-256. The hash covers the
/// canonical JSON (sorted keys, no whitespace) of the `EntitlementsSnapshot`.
#[must_use]
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
    format!("sha256:{result:x}")
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

// ===========================================================================
// Issuer / audience expectations (Gate A6: config, not code)
// ===========================================================================

/// Compiled-in DEFAULT issuer allow-list. Overridden per environment via
/// `JWT_ALLOWED_ISSUERS` (comma-separated) — see [`allowed_issuers`].
pub const ALLOWED_ISSUERS: &[&str] = &[
    "https://sesame-idam.example.com",
    "https://idam.example.com",
];

/// Compiled-in DEFAULT audience allow-list: the platform audiences plus the
/// per-service audiences each consumer's `security.jwks.*.aud` declares.
/// Overridden per environment via `JWT_EXPECTED_AUDIENCES` (comma-separated)
/// — see [`expected_audiences`].
pub const EXPECTED_AUDIENCE: &[&str] = &[
    "sesame-idam",
    "api",
    "frontend",
    "mobile",
    "identity-login",
    "identity-session",
    "authz-core",
    "org-mgmt",
    "identity-user-mgmt",
    "api-keys",
];

fn env_list(name: &str, defaults: &[&str]) -> Vec<String> {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => v
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => defaults.iter().map(|s| (*s).to_string()).collect(),
    }
}

/// Effective issuer allow-list: `JWT_ALLOWED_ISSUERS` env (comma-separated)
/// when set, else [`ALLOWED_ISSUERS`]. Read once per process.
pub fn allowed_issuers() -> &'static [String] {
    static ISSUERS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    ISSUERS.get_or_init(|| env_list("JWT_ALLOWED_ISSUERS", ALLOWED_ISSUERS))
}

/// Effective audience allow-list: `JWT_EXPECTED_AUDIENCES` env
/// (comma-separated) when set, else [`EXPECTED_AUDIENCE`]. Read once per
/// process.
pub fn expected_audiences() -> &'static [String] {
    static AUDIENCES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    AUDIENCES.get_or_init(|| env_list("JWT_EXPECTED_AUDIENCES", EXPECTED_AUDIENCE))
}
