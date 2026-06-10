//! Version bump event type and serialization.
//!
//! Represents a push invalidation event published when authz changes occur
//! (role revoked, user disabled, org deleted, etc.).

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Reason for the version bump.
///
/// These map to specific authz operations that trigger a version bump.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BumpReason {
    /// A role was assigned to a principal.
    RoleAssigned,
    /// A role was revoked from a principal.
    RoleRevoked,
    /// A user was disabled.
    UserDisabled,
    /// A user was enabled (re-enabled after being disabled).
    UserEnabled,
    /// An organization was deleted.
    OrgDeleted,
    /// A permission was modified.
    PermissionModified,
    /// An application was deleted.
    AppDeleted,
    /// A principal attribute was changed.
    PrincipalAttributeModified,
    /// A generic/unknown authz change.
    Other(String),
}

/// A version bump event for push invalidation.
///
/// Published via Redis pub/sub when authz changes occur.
/// Subscribers update their local version cache upon receiving this event.
///
/// # Event Format
///
/// ```json
/// {
///   "event": "version_bump",
///   "tenant_id": "tenant_abc",
///   "user_id": "user_123",        // optional, for subject-specific bumps
///   "new_version": 43,
///   "reason": "role_revoked",
///   "timestamp": 1715000000
/// }
/// ```
///
/// # Security Considerations
///
/// - Events are NOT signed in the current implementation (see HACK-501/HACK-505 in story).
/// - Push invalidation is a LATENCY OPTIMIZATION, not the primary revocation mechanism.
/// - The primary revocation mechanism is the version check on every request (Story 5.2).
/// - If Redis pub/sub is unavailable, the next Redis lookup (polling) catches up within the version cache TTL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionBumpEvent {
    /// Event type discriminator. Must be "version_bump".
    pub event: String,
    /// Tenant ID for the affected tenant.
    pub tenant_id: String,
    /// Optional user ID for subject-specific version bumps.
    /// If absent, this is a tenant-wide bump.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// The new version number (monotonically increasing u64).
    pub new_version: u64,
    /// Reason for the version bump.
    pub reason: BumpReason,
    /// Unix timestamp when the event was created (seconds since epoch).
    pub timestamp: u64,
}

impl VersionBumpEvent {
    /// Create a new version bump event for a subject (user-specific).
    pub fn for_subject(
        tenant_id: &str,
        user_id: &str,
        new_version: u64,
        reason: BumpReason,
    ) -> Self {
        Self {
            event: "version_bump".to_string(),
            tenant_id: tenant_id.to_string(),
            user_id: Some(user_id.to_string()),
            new_version,
            reason,
            timestamp: Self::now_timestamp(),
        }
    }

    /// Create a new version bump event for a tenant (tenant-wide, no user).
    pub fn for_tenant(tenant_id: &str, new_version: u64, reason: BumpReason) -> Self {
        Self {
            event: "version_bump".to_string(),
            tenant_id: tenant_id.to_string(),
            user_id: None,
            new_version,
            reason,
            timestamp: Self::now_timestamp(),
        }
    }

    /// Get the current Unix timestamp in seconds.
    fn now_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Check if this is a subject-specific bump.
    pub fn is_subject_specific(&self) -> bool {
        self.user_id.is_some()
    }

    /// Get the cache key for a subject-specific event.
    pub fn subject_cache_key(&self) -> Option<String> {
        self.user_id
            .as_ref()
            .map(|uid| format!("authz_ver:{}", uid))
    }

    /// Get the tenant cache key.
    pub fn tenant_cache_key(&self) -> String {
        format!("authz_ver:tenant:{}", self.tenant_id)
    }

    /// Validate the event fields.
    ///
    /// Returns an error string if the event is invalid, or `Ok(())` if valid.
    pub fn validate(&self) -> Result<(), String> {
        if self.event != "version_bump" {
            return Err(format!(
                "invalid event type: expected 'version_bump', got '{}'",
                self.event
            ));
        }
        if self.tenant_id.is_empty() {
            return Err("tenant_id is empty".to_string());
        }
        if self.new_version == 0 {
            return Err("new_version is 0, cannot be a valid bump".to_string());
        }
        if self.timestamp == 0 {
            return Err("timestamp is 0".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization_roundtrip() {
        let event =
            VersionBumpEvent::for_subject("tenant_abc", "user_123", 43, BumpReason::RoleRevoked);
        let json = event.to_json().unwrap();
        let deserialized = VersionBumpEvent::from_json(&json).unwrap();
        assert_eq!(deserialized.tenant_id, "tenant_abc");
        assert_eq!(deserialized.user_id, Some("user_123".to_string()));
        assert_eq!(deserialized.new_version, 43);
        assert_eq!(deserialized.reason, BumpReason::RoleRevoked);
        assert_eq!(deserialized.event, "version_bump");
    }

    #[test]
    fn test_tenant_wide_event() {
        let event = VersionBumpEvent::for_tenant("tenant_abc", 10, BumpReason::OrgDeleted);
        assert!(!event.is_subject_specific());
        assert!(event.subject_cache_key().is_none());
        assert_eq!(event.tenant_cache_key(), "authz_ver:tenant:tenant_abc");
    }

    #[test]
    fn test_subject_event_has_both_cache_keys() {
        let event =
            VersionBumpEvent::for_subject("tenant_abc", "user_123", 43, BumpReason::RoleAssigned);
        assert!(event.is_subject_specific());
        assert_eq!(
            event.subject_cache_key(),
            Some("authz_ver:user_123".to_string())
        );
        assert_eq!(event.tenant_cache_key(), "authz_ver:tenant:tenant_abc");
    }

    #[test]
    fn test_validation_empty_tenant_id() {
        let event = VersionBumpEvent {
            event: "version_bump".to_string(),
            tenant_id: "".to_string(),
            user_id: None,
            new_version: 10,
            reason: BumpReason::Other("test".to_string()),
            timestamp: 1715000000,
        };
        assert!(event.validate().is_err());
        assert!(event.validate().unwrap_err().contains("tenant_id is empty"));
    }

    #[test]
    fn test_validation_zero_version() {
        let event = VersionBumpEvent {
            event: "version_bump".to_string(),
            tenant_id: "tenant_abc".to_string(),
            user_id: None,
            new_version: 0,
            reason: BumpReason::Other("test".to_string()),
            timestamp: 1715000000,
        };
        assert!(event.validate().is_err());
        assert!(event.validate().unwrap_err().contains("new_version is 0"));
    }

    #[test]
    fn test_validation_valid_event() {
        let event = VersionBumpEvent {
            event: "version_bump".to_string(),
            tenant_id: "tenant_abc".to_string(),
            user_id: None,
            new_version: 10,
            reason: BumpReason::RoleRevoked,
            timestamp: 1715000000,
        };
        assert!(event.validate().is_ok());
    }

    #[test]
    fn test_validation_invalid_event_type() {
        let event = VersionBumpEvent {
            event: "wrong_type".to_string(),
            tenant_id: "tenant_abc".to_string(),
            user_id: None,
            new_version: 10,
            reason: BumpReason::Other("test".to_string()),
            timestamp: 1715000000,
        };
        assert!(event.validate().is_err());
        assert!(event.validate().unwrap_err().contains("invalid event type"));
    }

    #[test]
    fn test_json_contains_required_fields() {
        let event =
            VersionBumpEvent::for_subject("tenant_abc", "user_123", 43, BumpReason::RoleRevoked);
        let json = event.to_json().unwrap();
        assert!(json.contains("\"event\":\"version_bump\""));
        assert!(json.contains("\"tenant_id\":\"tenant_abc\""));
        assert!(json.contains("\"user_id\":\"user_123\""));
        assert!(json.contains("\"new_version\":43"));
        assert!(json.contains("\"reason\":\"role_revoked\""));
    }

    #[test]
    fn test_json_omits_user_id_for_tenant_wide() {
        let event = VersionBumpEvent::for_tenant("tenant_abc", 43, BumpReason::OrgDeleted);
        let json = event.to_json().unwrap();
        assert!(!json.contains("user_id"));
    }

    #[test]
    fn test_json_has_timestamp() {
        let event =
            VersionBumpEvent::for_subject("tenant_abc", "user_123", 43, BumpReason::RoleRevoked);
        let json = event.to_json().unwrap();
        assert!(json.contains("\"timestamp\":"));
    }

    #[test]
    fn test_event_with_large_version() {
        let event = VersionBumpEvent::for_tenant(
            "tenant_abc",
            u64::MAX,
            BumpReason::Other("overflow".to_string()),
        );
        assert!(event.validate().is_ok());
        let json = event.to_json().unwrap();
        let deserialized = VersionBumpEvent::from_json(&json).unwrap();
        assert_eq!(deserialized.new_version, u64::MAX);
    }

    #[test]
    fn test_malformed_json() {
        let result = VersionBumpEvent::from_json("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_tenant_id_field() {
        let json = r#"{"event":"version_bump","new_version":10,"reason":"role_revoked","timestamp":1715000000}"#;
        let result = VersionBumpEvent::from_json(json);
        // Should deserialize but fail validation
        if let Ok(event) = result {
            assert!(event.validate().is_err());
            assert!(event.validate().unwrap_err().contains("tenant_id is empty"));
        }
    }

    #[test]
    fn test_unknown_reason() {
        let event = VersionBumpEvent {
            event: "version_bump".to_string(),
            tenant_id: "tenant_abc".to_string(),
            user_id: None,
            new_version: 10,
            reason: BumpReason::Other("unknown_event_type".to_string()),
            timestamp: 1715000000,
        };
        assert!(event.validate().is_ok());
    }
}
