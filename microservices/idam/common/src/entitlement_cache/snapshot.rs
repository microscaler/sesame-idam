//! Permission types and entitlement snapshot data structure.
//!
//! `Permission` represents a single access right (action on resource).
//! `EntitlementSnapshot` holds a complete, pre-computed ACL for a user/org pair.

use serde::{Deserialize, Serialize};

/// A single permission (action on a resource).
///
/// Represents one grant in the ACL — e.g., "read:documents" or "admin:users".
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Permission {
    /// The action: "read", "write", "delete", "admin", etc.
    pub action: String,
    /// The resource: "documents", "users", "reports", etc.
    pub resource: String,
}

impl Permission {
    /// Create a new permission.
    #[must_use]
    pub fn new(action: &str, resource: &str) -> Self {
        Self {
            action: action.to_string(),
            resource: resource.to_string(),
        }
    }

    /// Check if this permission grants a specific action on a specific resource.
    #[must_use]
    pub fn matches(&self, action: &str, resource: &str) -> bool {
        self.action == action && self.resource == resource
    }

    /// Check if this permission is a high-risk permission.
    #[must_use]
    pub fn is_high_risk(&self) -> bool {
        is_high_risk_action(&self.action)
    }
}

/// Check if an action is considered high-risk.
#[must_use]
pub fn is_high_risk_action(action: &str) -> bool {
    matches_high_risk(action)
}

/// Set of actions considered high-risk for entitlement caching purposes.
///
/// High-risk permissions require shorter TTLs (30 seconds) to minimize
/// the window for permission escalation attacks (HACK-751).
fn matches_high_risk(action: &str) -> bool {
    matches!(
        action,
        "admin"
            | "delete"
            | "superadmin"
            | "destroy"
            | "deactivate"
            | "impersonate"
            | "modify_permissions"
            | "manage_roles"
            | "system_config"
    )
}

/// Entitlement complexity classification.
///
/// Determines the TTL for caching — more complex entitlements change more
/// frequently and need shorter TTLs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntitlementComplexity {
    /// Static roles (no org, no custom logic) — longest TTL.
    Static,
    /// Role + org assignment — moderate TTL.
    RoleOrg,
    /// Role + org + custom logic — short TTL.
    Custom,
    /// Fully dynamic/computed entitlements — shortest TTL.
    Dynamic,
}

impl EntitlementComplexity {
    /// Get the default TTL in seconds for this complexity level.
    #[must_use]
    pub fn default_ttl_seconds(&self) -> f64 {
        match self {
            EntitlementComplexity::Static => 300.0,
            EntitlementComplexity::RoleOrg => 120.0,
            EntitlementComplexity::Custom => 60.0,
            EntitlementComplexity::Dynamic => 30.0,
        }
    }

    /// Check if any permission in this complexity level is high-risk.
    #[must_use]
    pub fn is_complexity_high_risk(&self) -> bool {
        // Dynamic and Custom complexity levels are more likely to contain high-risk perms
        matches!(
            self,
            EntitlementComplexity::Custom | EntitlementComplexity::Dynamic
        )
    }
}

/// A pre-computed ACL snapshot for a user/org pair.
///
/// This is the data stored in the cache when authz-core resolves an
/// `entitlements_ref`. It contains the full list of permissions that
/// can be evaluated locally without another authz-core call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntitlementSnapshot {
    /// The user ID this snapshot belongs to.
    pub user_id: String,
    /// The organization ID this snapshot belongs to.
    pub org_id: String,
    /// The full list of permissions.
    pub permissions: Vec<Permission>,
    /// The complexity classification of this entitlement set.
    pub complexity: EntitlementComplexity,
    /// The time this snapshot was computed (wall-clock).
    pub computed_at: String,
}

impl EntitlementSnapshot {
    /// Create a new entitlement snapshot.
    #[must_use]
    pub fn new(
        user_id: &str,
        org_id: &str,
        permissions: Vec<Permission>,
        complexity: EntitlementComplexity,
    ) -> Self {
        Self {
            user_id: user_id.to_string(),
            org_id: org_id.to_string(),
            permissions,
            complexity,
            computed_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Check if this snapshot contains any high-risk permissions.
    #[must_use]
    pub fn contains_high_risk(&self) -> bool {
        self.permissions.iter().any(Permission::is_high_risk)
    }

    /// Check if this snapshot grants a specific action on a specific resource.
    #[must_use]
    pub fn has_permission(&self, action: &str, resource: &str) -> bool {
        self.permissions.iter().any(|p| p.matches(action, resource))
    }

    /// Get the TTL in seconds for this snapshot based on its complexity.
    #[must_use]
    pub fn ttl_seconds(&self) -> f64 {
        self.complexity.default_ttl_seconds()
    }

    /// Get the estimated serialized size in bytes (JSON).
    #[must_use]
    pub fn serialized_size_bytes(&self) -> usize {
        serde_json::to_string(self).map_or(0, |s| s.len())
    }
}

/// Result of a cache lookup, carrying both the snapshot and whether it was a hit.
#[derive(Debug, Clone)]
pub struct CacheLookupResult {
    /// The entitlement snapshot.
    pub snapshot: EntitlementSnapshot,
    /// Whether this was a cache hit (true) or miss (false).
    pub is_hit: bool,
}

impl CacheLookupResult {
    #[must_use]
    pub fn hit(snapshot: EntitlementSnapshot) -> Self {
        Self {
            snapshot,
            is_hit: true,
        }
    }

    #[must_use]
    pub fn miss(snapshot: EntitlementSnapshot) -> Self {
        Self {
            snapshot,
            is_hit: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_creation() {
        let perm = Permission::new("read", "documents");
        assert_eq!(perm.action, "read");
        assert_eq!(perm.resource, "documents");
    }

    #[test]
    fn test_permission_matches() {
        let perm = Permission::new("write", "reports");
        assert!(perm.matches("write", "reports"));
        assert!(!perm.matches("read", "reports"));
        assert!(!perm.matches("write", "users"));
    }

    #[test]
    fn test_permission_high_risk() {
        assert!(Permission::new("admin", "users").is_high_risk());
        assert!(Permission::new("delete", "records").is_high_risk());
        assert!(!Permission::new("read", "documents").is_high_risk());
        assert!(!Permission::new("write", "documents").is_high_risk());
    }

    #[test]
    fn test_high_risk_action() {
        assert!(is_high_risk_action("admin"));
        assert!(is_high_risk_action("delete"));
        assert!(is_high_risk_action("superadmin"));
        assert!(is_high_risk_action("impersonate"));
        assert!(is_high_risk_action("manage_roles"));
        assert!(!is_high_risk_action("read"));
        assert!(!is_high_risk_action("write"));
        assert!(!is_high_risk_action("update"));
    }

    #[test]
    fn test_entitlement_complexity_ttl() {
        assert_eq!(EntitlementComplexity::Static.default_ttl_seconds(), 300.0);
        assert_eq!(EntitlementComplexity::RoleOrg.default_ttl_seconds(), 120.0);
        assert_eq!(EntitlementComplexity::Custom.default_ttl_seconds(), 60.0);
        assert_eq!(EntitlementComplexity::Dynamic.default_ttl_seconds(), 30.0);
    }

    #[test]
    fn test_entitlement_complexity_high_risk() {
        assert!(EntitlementComplexity::Custom.is_complexity_high_risk());
        assert!(EntitlementComplexity::Dynamic.is_complexity_high_risk());
        assert!(!EntitlementComplexity::Static.is_complexity_high_risk());
        assert!(!EntitlementComplexity::RoleOrg.is_complexity_high_risk());
    }

    #[test]
    fn test_entitlement_snapshot_creation() {
        let perms = vec![
            Permission::new("read", "documents"),
            Permission::new("write", "documents"),
        ];
        let snap =
            EntitlementSnapshot::new("user_123", "org_456", perms, EntitlementComplexity::Static);
        assert_eq!(snap.user_id, "user_123");
        assert_eq!(snap.org_id, "org_456");
        assert_eq!(snap.permissions.len(), 2);
        assert_eq!(snap.ttl_seconds(), 300.0);
    }

    #[test]
    fn test_snapshot_has_permission() {
        let perms = vec![
            Permission::new("read", "documents"),
            Permission::new("admin", "users"),
        ];
        let snap =
            EntitlementSnapshot::new("user_123", "org_456", perms, EntitlementComplexity::Static);
        assert!(snap.has_permission("read", "documents"));
        assert!(snap.has_permission("admin", "users"));
        assert!(!snap.has_permission("delete", "users"));
    }

    #[test]
    fn test_snapshot_contains_high_risk() {
        let perms = vec![
            Permission::new("read", "documents"),
            Permission::new("admin", "users"),
        ];
        let snap =
            EntitlementSnapshot::new("user_123", "org_456", perms, EntitlementComplexity::Static);
        assert!(snap.contains_high_risk());

        let perms2 = vec![Permission::new("read", "documents")];
        let snap2 =
            EntitlementSnapshot::new("user_123", "org_456", perms2, EntitlementComplexity::Static);
        assert!(!snap2.contains_high_risk());
    }

    #[test]
    fn test_snapshot_serialization() {
        let perms = vec![
            Permission::new("read", "documents"),
            Permission::new("write", "reports"),
        ];
        let snap =
            EntitlementSnapshot::new("user_123", "org_456", perms, EntitlementComplexity::RoleOrg);
        let json = serde_json::to_string(&snap).unwrap();
        let deserialized: EntitlementSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.user_id, snap.user_id);
        assert_eq!(deserialized.permissions.len(), snap.permissions.len());
        assert_eq!(deserialized.complexity, EntitlementComplexity::RoleOrg);
    }

    #[test]
    fn test_cache_lookup_result_hit() {
        let perms = vec![Permission::new("read", "docs")];
        let snap = EntitlementSnapshot::new("u1", "o1", perms, EntitlementComplexity::Static);
        let result = CacheLookupResult::hit(snap.clone());
        assert!(result.is_hit);
        assert_eq!(result.snapshot.user_id, "u1");
    }

    #[test]
    fn test_cache_lookup_result_miss() {
        let perms = vec![Permission::new("read", "docs")];
        let snap = EntitlementSnapshot::new("u1", "o1", perms, EntitlementComplexity::Static);
        let result = CacheLookupResult::miss(snap.clone());
        assert!(!result.is_hit);
    }

    #[test]
    fn test_permission_special_characters() {
        let perm = Permission::new("read:v2", "documents:v1");
        assert!(perm.matches("read:v2", "documents:v1"));
        assert!(!perm.matches("read", "documents"));
    }

    #[test]
    fn test_permission_unicode() {
        let perm = Permission::new("读取", "文档");
        assert!(perm.matches("读取", "文档"));
        assert_eq!(perm.action, "读取");
        assert_eq!(perm.resource, "文档");
    }

    #[test]
    fn test_permission_empty_values() {
        let perm = Permission::new("", "");
        assert!(perm.matches("", ""));
        assert!(!perm.matches("read", ""));
        assert!(!perm.is_high_risk());
    }

    #[test]
    fn test_permission_long_strings() {
        let long_action = "a".repeat(500);
        let long_resource = "r".repeat(500);
        let perm = Permission::new(&long_action, &long_resource);
        assert_eq!(perm.action.len(), 500);
        assert_eq!(perm.resource.len(), 500);
        assert!(perm.matches(&long_action, &long_resource));
    }

    #[test]
    fn test_entitlement_snapshot_empty_permissions() {
        let snap =
            EntitlementSnapshot::new("user_123", "org_456", vec![], EntitlementComplexity::Static);
        assert!(snap.permissions.is_empty());
        assert!(!snap.contains_high_risk());
        assert!(!snap.has_permission("read", "documents"));
    }
}
