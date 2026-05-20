//! # sesame-jwt-common-path
//!
//! JWT common-path authorization middleware for Sesame-IDAM.
//!
//! This crate implements the JWT middleware for the hybrid authorization model
//! (Epic 4). It sits between BRRTRouter's router and handlers, validating JWTs
//! and evaluating local policy from claims for `jwt-only` routes.
//!
//! ## Architecture
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
//! ## Route Categories
//!
//! - **`jwt-only`**: All authz decisions from JWT claims — no authz-core call needed
//! - **`jwt-with-fallback`**: JWT handles common path, online fallback for edge cases
//! - **`online-only`**: All decisions require online evaluation via authz-core
//!
//! ## Security
//!
//! - HACK-401: Tenant validation MUST compare claims.tenant_id against X-Tenant-ID
//! - HACK-403: ALL routes must validate X-Tenant-ID presence
//! - HACK-405: NEVER fail open — all errors reject (503/401/403)
//! - HACK-407: Token expiry check BEFORE expensive JWKS operations
//!
//! ## Usage
//!
//! ```rust,ignore
//! use sesame_jwt_common_path::JwtAuthMiddleware;
//! use std::sync::Arc;
//!
//! let policies = RoutePolicyStore::load_from_yaml("config/routes.yaml").unwrap();
//! let middleware = JwtAuthMiddleware::new(Arc::new(policies));
//!
//! // In your handler:
//! let decision = middleware.validate_and_authorize(&request).await?;
//! ```

pub mod auth_decision;
pub mod jwt_validator;
pub mod local_policy;
pub mod middleware;
pub mod route_policy;

// Re-export public types for convenience
pub use auth_decision::{AuthDecision, AuthError};
pub use jwt_validator::{extract_bearer_token, parse_claims, pre_validate_expiry};
pub use local_policy::evaluate_local_policy;
pub use middleware::JwtAuthMiddleware;
pub use route_policy::{RouteAuthCategory, RoutePolicy, RoutePolicyStore};
