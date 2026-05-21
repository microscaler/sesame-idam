//! Shared denylist cache for JTI (JWT ID) revocation caching.
//!
//! Provides an in-memory denylist cache that reduces Redis lookup overhead
//! for revoked token identification (JTI checks). The cache uses dynamic TTL
//! based on token expiry with a 5-minute hard cap.
//!
//! # Key Design Decisions
//!
//! - **Redis is the source of truth**: The cache is a performance layer only.
//!   On cache miss, Redis is always consulted.
//! - **Fail-closed**: If Redis is unavailable, tokens are rejected to maintain
//!   security.
//! - **Dynamic TTL**: Cache entries live until the revoked token's `exp` would
//!   expire, capped at 5 minutes.
//! - **Jitter**: Randomized TTL jitter prevents thundering herd on Redis.
//! - **LRU eviction**: When the cache reaches 10,000 entries, oldest entries
//!   are evicted first.
//!
//! # Security Considerations
//!
//! - HACK-741: Redis entries MUST NOT expire for revocation — Redis is the
//!   authoritative source of truth.
//! - HACK-742: Max entries limit enforced (10,000 per instance) to prevent
//!   JTI flooding attacks.
//! - HACK-743: TTL jitter prevents cache miss storms when many entries expire
//!   simultaneously.
//!
//! # Example
//!
//! ```no_run
//! use sesame_denylist::{DenylistCache, DenylistConfig};
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = DenylistConfig::default();
//!     let cache = DenylistCache::new(config);
//!     
//!     // In a real service, you'd inject a Redis client:
//!     // let is_revoked = cache.is_revoked("jti-abc123", &redis_client).await;
//! }
//! ```

mod cache;
mod config;
mod metrics;

pub use cache::{DenylistCache, DenylistResult};
pub use config::DenylistConfig;
pub use metrics::{register_denylist_metrics, DenylistMetrics};
