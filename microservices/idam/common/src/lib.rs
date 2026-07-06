//! Shared JWT claim types, validation, builder, middleware, JWKS cache, audit logging,
//! denylist caching, entitlement snapshot caching, JWT common-path authorization,
//! and token versioning for Sesame-IDAM microservices.
//!
//! This crate provides:
//! - `jwt` module — JWT claim structures (`AccessClaims`, `SesameAuthzClaims`, `ActorClaim`)
//! - `middleware` module — JWT Common-Path Authorization Middleware for `BRRTRouter`
//! - `jwks_cache` module — JWKS cache with background refresh, stale tolerance, and security protections
//! - `fallback_cache` module — Selective online fallback with Redis caching (Story 4.3)
//! - `audit` module — Security audit logging (structured JSON, priority queues, rate limiting)
//! - `denylist` module — JTI revocation cache (in-memory Redis layer)
//! - `entitlement_cache` module — Entitlement snapshot cache with TTL eviction
//! - `jwt_common_path` module — JWT validation + local policy evaluation (hybrid authz Epic 4)
//! - `token_versioning` module — Version bump events, pub/sub publisher/subscriber, version store
//!
//! The middleware enables fast-path authorization for `jwt-only` routes without
//! calling authz-core, by evaluating policy locally from JWT claims.

// Existing modules
pub mod config;
pub mod dpop;
pub mod fallback_cache;
pub mod jwks_cache;
pub mod jwt;
pub mod middleware;

// Consolidated sibling crate modules
pub mod audit;
pub mod denylist;
pub mod entitlement_cache;
pub mod jwt_common_path;
pub mod token_versioning;

// Re-export from existing modules
pub use jwt::{
    AccessClaims, AccessClaimsBuilder, ActorClaim, JwtError, JwtValidationError, SesameAuthzClaims,
    SesameAuthzClaimsBuilder, ALLOWED_ISSUERS, EXPECTED_AUDIENCE,
};

// Re-export from audit module
pub use audit::{
    allowed_event_types, generate_key, is_valid_event_type, sign_entry, verify_entry, AuditActor,
    AuditEmitter, AuditEvent, AuditEventType, AuditLevel, AuditLogEntry, AuditLogEntryBuilder,
    AuditMetrics, AuditQueue, RateLimitConfig, RateLimiter,
};

// Re-export from denylist module
pub use denylist::{
    register_denylist_metrics, DenylistCache, DenylistConfig, DenylistMetrics, DenylistResult,
};

// Re-export from entitlement_cache module
pub use entitlement_cache::{
    CacheConfig, CacheError, CacheLookupResult, EntitlementComplexity, EntitlementSnapshot,
    EntitlementSnapshotCache, Permission,
};

// Re-export from jwt_common_path module
pub use jwt_common_path::{
    compute_jkt, evaluate_local_policy, extract_bearer_token, generate_ed25519_keypair,
    generate_p256_keypair, parse_claims, pre_validate_expiry, verify_dpop_proof, AuthDecision,
    AuthError, DpopConfirmation, DpopError, DpopJwk, InMemoryProofStore, JwtAuthMiddleware,
    RouteAuthCategory, RoutePolicy, RoutePolicyStore,
};

// Re-export from token_versioning module
pub use token_versioning::{
    subject_key, tenant_key, BumpReason, VersionBumpEvent, VersionBumpPublisher,
    VersionBumpSubscriber, VersionStore, VersionStoreConfig,
};
