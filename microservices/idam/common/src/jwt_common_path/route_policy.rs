//! # Route Policy
//!
//! Defines the route classification schema and `RoutePolicyStore` that the JWT middleware uses
//! to determine which authorization path to take for each endpoint.
//!
//! ## Route Categories
//!
//! - **`JwtOnly`**: All authz decisions made from JWT claims alone — no authz-core call needed.
//! - **`JwtWithFallback`**: JWT validates common path; online fallback for edge cases (cached 5-30s).
//! - **`OnlineOnly`**: All decisions require online evaluation via authz-core.
//!
//! ## Path Matching
//!
//! Route lookup uses **exact path matching** on the routed/template path (e.g., `/admin/users/{id}`),
//! **not** the raw HTTP request path (e.g., `/admin/users/123`). This prevents prefix bypass attacks.
//!
//! ## Security
//!
//! - Duplicate path+method entries in YAML cause startup failure (HACK-209).
//! - Wildcard classifications are not supported — all routes must be explicitly classified.
//! - Admin routes must NEVER be classified as `jwt-only` (HACK-201).

use std::collections::HashMap;
use std::fs;

use serde::{Deserialize, Serialize};

use super::auth_decision::AuthError;

// ---------------------------------------------------------------------------
// RouteAuthCategory — classification of how each route is authorized
// ---------------------------------------------------------------------------

/// Authorization category for a route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteAuthCategory {
    /// All authorization decisions can be made from JWT claims alone.
    /// No authz-core call is needed.
    JwtOnly,

    /// JWT handles the common path; online fallback for edge cases.
    /// Cached for `cache_ttl_secs` (5-30 seconds).
    #[serde(rename_all = "snake_case")]
    JwtWithFallback {
        /// Cache TTL in seconds for the online fallback result.
        cache_ttl_secs: u64,
        /// Whether this route requires token version check against cached version.
        requires_fresh_version: bool,
    },

    /// All authorization decisions require online evaluation.
    /// No JWT common-path optimization — always calls authz-core.
    OnlineOnly,
}

impl std::fmt::Display for RouteAuthCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouteAuthCategory::JwtOnly => write!(f, "jwt-only"),
            RouteAuthCategory::JwtWithFallback { .. } => write!(f, "jwt-with-fallback"),
            RouteAuthCategory::OnlineOnly => write!(f, "online-only"),
        }
    }
}

impl Default for RouteAuthCategory {
    fn default() -> Self {
        RouteAuthCategory::JwtWithFallback {
            cache_ttl_secs: 30,
            requires_fresh_version: false,
        }
    }
}

impl RouteAuthCategory {
    /// Returns true if this category allows local policy evaluation without
    /// calling authz-core.
    pub fn is_jwt_only(&self) -> bool {
        matches!(self, RouteAuthCategory::JwtOnly)
    }

    /// Returns the cache TTL if this category supports caching.
    pub fn cache_ttl_secs(&self) -> Option<u64> {
        match self {
            RouteAuthCategory::JwtWithFallback { cache_ttl_secs, .. } => Some(*cache_ttl_secs),
            _ => None,
        }
    }

    /// Returns true if this category requires token version checking.
    pub fn requires_fresh_version(&self) -> bool {
        match self {
            RouteAuthCategory::JwtWithFallback {
                requires_fresh_version,
                ..
            } => *requires_fresh_version,
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// RoutePolicy — classification for a specific route+method combination
// ---------------------------------------------------------------------------

/// A route policy that defines how a specific path+method combination is authorized.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePolicy {
    /// The route path (template, e.g., `/admin/users/{id}`).
    pub path: String,
    /// HTTP methods this policy applies to.
    pub methods: Vec<String>,
    /// Authorization category.
    pub category: RouteAuthCategory,
    /// Human-readable description of why this route is classified this way.
    pub description: String,
}

impl RoutePolicy {
    /// Create a new route policy.
    #[must_use]
    pub fn new(
        path: impl Into<String>,
        methods: Vec<String>,
        category: RouteAuthCategory,
        description: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            methods,
            category,
            description: description.into(),
        }
    }

    /// Create a lookup key for this policy: `"path:METHOD"`.
    #[must_use]
    pub fn lookup_key(&self, method: &str) -> String {
        format!("{}:{}", self.path, method)
    }
}

// ---------------------------------------------------------------------------
// RoutePolicyStore — in-memory store for fast route lookup
// ---------------------------------------------------------------------------

/// In-memory store of route policies with fast O(1) lookup by path+method.
///
/// Policies are loaded from a YAML file at startup. The store builds an internal
/// `HashMap` for fast lookup using `"path:METHOD"` keys.
///
/// # Security
///
/// - Duplicate path+method entries cause a startup failure (not silently ignored).
/// - The store does NOT support wildcard matching — all routes must be explicitly classified.
pub struct RoutePolicyStore {
    policies: Vec<RoutePolicy>,
    lookup: HashMap<String, RoutePolicy>,
}

impl RoutePolicyStore {
    /// Load route policies from a YAML file.
    ///
    /// Returns `AuthError::InternalError` if the file cannot be read or parsed.
    /// Duplicate path+method entries will be rejected.
    ///
    /// # YAML Format
    ///
    /// ```yaml
    /// route_policies:
    ///   - path: "/admin/users/me"
    ///     methods: ["GET"]
    ///     category: "jwt-only"
    ///     description: "Self-service read, ownership from JWT"
    ///   - path: "/admin/users/me/preferences"
    ///     methods: ["PUT", "PATCH"]
    ///     category: "jwt-with-fallback"
    ///     cache_ttl_secs: 30
    ///     requires_fresh_version: false
    ///     description: "Low-risk write, business validation stays online"
    /// ```
    pub fn load_from_yaml(path: &str) -> Result<Self, AuthError> {
        let content = fs::read_to_string(path).map_err(|e| {
            AuthError::InternalError(format!("Failed to read route policy file: {e}"))
        })?;

        let config: RoutePolicyConfig = serde_yaml::from_str(&content).map_err(|e| {
            AuthError::InternalError(format!("Failed to parse route policy YAML: {e}"))
        })?;

        Self::from_config(config)
    }

    /// Build a route policy store from a `RoutePolicyConfig`.
    pub fn from_config(config: RoutePolicyConfig) -> Result<Self, AuthError> {
        let mut lookup: HashMap<String, RoutePolicy> = HashMap::new();
        let mut policies = Vec::new();

        for policy_cfg in config.route_policies {
            let category = policy_cfg.to_route_category();

            for method in &policy_cfg.methods {
                let key = format!("{}:{}", policy_cfg.path, method);

                if lookup.contains_key(&key) {
                    return Err(AuthError::InternalError(format!(
                        "Duplicate path+method entry: `{}` (first defined with category {:?}, this one with {:?})",
                        policy_cfg.path,
                        lookup.get(&key).unwrap().category,
                        category
                    )));
                }

                let policy = RoutePolicy {
                    path: policy_cfg.path.clone(),
                    methods: policy_cfg.methods.clone(),
                    category: category.clone(),
                    description: policy_cfg.description.clone(),
                };

                lookup.insert(key, policy.clone());
                policies.push(policy);
            }
        }

        Ok(Self { policies, lookup })
    }

    /// Look up a route policy by path and method.
    ///
    /// Uses **exact path matching** on the routed/template path.
    /// Returns `None` if no policy matches.
    #[must_use]
    pub fn get_policy(&self, path: &str, method: &str) -> Option<&RoutePolicy> {
        let key = format!("{}:{}", path, method);
        self.lookup.get(&key)
    }

    /// Get the default category for routes not in the config.
    ///
    /// **FAIL-SAFE**: The default is `jwt-with-fallback`, which ensures
    /// the online fallback IS called for unknown routes (fail-closed).
    #[must_use]
    pub fn default_category() -> RouteAuthCategory {
        RouteAuthCategory::default()
    }

    /// Returns the number of policies in the store.
    #[must_use]
    pub fn len(&self) -> usize {
        self.policies.len()
    }

    /// Returns true if the store has no policies.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.policies.is_empty()
    }

    /// Iterate over all policies.
    #[must_use]
    pub fn iter(&self) -> impl Iterator<Item = &RoutePolicy> {
        self.policies.iter()
    }

    /// Get the category for a route+method, falling back to the default.
    #[must_use]
    pub fn get_category(&self, path: &str, method: &str) -> RouteAuthCategory {
        self.get_policy(path, method)
            .map(|p| p.category.clone())
            .unwrap_or_else(RouteAuthCategory::default)
    }
}

impl Default for RoutePolicyStore {
    /// Creates an empty store. Useful for testing.
    fn default() -> Self {
        Self {
            policies: Vec::new(),
            lookup: HashMap::new(),
        }
    }
}

impl RoutePolicyStore {
    /// Test helper: build a RoutePolicyStore directly from policies and lookup map.
    #[cfg(test)]
    pub(crate) fn from_parts(
        policies: Vec<RoutePolicy>,
        lookup: std::collections::HashMap<String, RoutePolicy>,
    ) -> Self {
        Self { policies, lookup }
    }
}

// ---------------------------------------------------------------------------
// YAML configuration types
// ---------------------------------------------------------------------------

/// Top-level YAML configuration.
#[derive(Debug, Deserialize)]
pub struct RoutePolicyConfig {
    pub route_policies: Vec<RoutePolicyYamlEntry>,
}

/// A single YAML entry for a route policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePolicyYamlEntry {
    pub path: String,
    pub methods: Vec<String>,
    pub category: RouteCategoryYaml,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_ttl_secs: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires_fresh_version: Option<bool>,
    pub description: String,
}

impl RoutePolicyYamlEntry {
    /// Convert the YAML representation to the runtime category.
    pub fn to_route_category(&self) -> RouteAuthCategory {
        match self.category {
            RouteCategoryYaml::JwtOnly => RouteAuthCategory::JwtOnly,
            RouteCategoryYaml::JwtWithFallback => RouteAuthCategory::JwtWithFallback {
                cache_ttl_secs: self.cache_ttl_secs.unwrap_or(30),
                requires_fresh_version: self.requires_fresh_version.unwrap_or(false),
            },
            RouteCategoryYaml::OnlineOnly => RouteAuthCategory::OnlineOnly,
        }
    }
}

/// YAML representation of a route category.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RouteCategoryYaml {
    JwtOnly,
    JwtWithFallback,
    OnlineOnly,
}

impl RouteCategoryYaml {
    /// Convert YAML category to runtime `RouteAuthCategory`.
    pub fn into_category(self) -> RouteAuthCategory {
        match self {
            RouteCategoryYaml::JwtOnly => RouteAuthCategory::JwtOnly,
            RouteCategoryYaml::JwtWithFallback => RouteAuthCategory::JwtWithFallback {
                cache_ttl_secs: 30,
                requires_fresh_version: false,
            },
            RouteCategoryYaml::OnlineOnly => RouteAuthCategory::OnlineOnly,
        }
    }
}