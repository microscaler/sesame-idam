//! Push invalidation publisher wrapper for authz-core.
//!
//! Provides a thread-safe, shared publisher that authz-changing controllers
//! call to emit version bump events to Redis pub/sub.
//!
//! # Usage
//!
//! Controllers call `publish_tenant()` or `publish_subject()` after performing
//! authz state changes. These are fire-and-forget — they spawn a may coroutine
//! via the `go!` macro and return immediately.
//!
//! # May Runtime
//!
//! All coroutines run on the may scheduler. The publish functions use
//! `may::go!` which schedules the closure on a may worker thread.
//! Inside the closure, Redis operations use the blocking sync API
//! (`redis::Commands` trait) — this blocks the coroutine's epoll wait until
//! the Redis response arrives, which is correct: the coroutine is idle while
//! waiting for network I/O, and other coroutines continue to be served.

use redis::Commands;
use sesame_common::token_versioning::events::{BumpReason, VersionBumpEvent};
use sesame_common::token_versioning::publisher::VERSION_BUMP_CHANNEL;

/// Shared publisher handle wrapped for sync callers.
pub struct PublisherWrapper {
    redis_url: String,
    hmac_secret: Vec<u8>,
}

impl PublisherWrapper {
    /// Create a new publisher wrapper.
    pub fn new(redis_url: &str, hmac_secret: Vec<u8>) -> Self {
        Self {
            redis_url: redis_url.to_string(),
            hmac_secret,
        }
    }

    /// Fire-and-forget: publish a tenant-wide version bump event.
    ///
    /// Spawns a may coroutine that publishes the event. The controller
    /// does not wait for the publish to complete — it is fire-and-forget.
    ///
    /// Inside the spawned closure, Redis uses blocking I/O. This is safe in
    /// may because the coroutine's epoll loop is idle while waiting for the
    /// socket — other coroutines continue to make progress.
    pub fn publish_tenant(&self, tenant_id: &str, new_version: u64, reason: BumpReason) {
        let url = self.redis_url.clone();
        let secret = self.hmac_secret.clone();
        let tenant_id = tenant_id.to_string();
        let reason_str = format!("{:?}", reason);

        may::go!(move || {
            let event = VersionBumpEvent::for_tenant(&tenant_id, new_version, reason);
            let json = match event.to_json() {
                Ok(j) => j,
                Err(e) => {
                    tracing::error!(error = %e, "failed to serialize event for publish_tenant");
                    return;
                }
            };

            let message = match Self::sign_event(&json, &secret) {
                Ok(msg) => msg,
                Err(e) => {
                    tracing::error!(error = %e, "failed to sign event for publish_tenant");
                    return;
                }
            };

            if let Err(e) = Self::publish_event(&url, &message) {
                tracing::error!(
                    tenant_id,
                    new_version,
                    reason = reason_str,
                    error = %e,
                    "failed to publish version bump event",
                );
            }
        });
    }

    /// Fire-and-forget: publish a subject-specific version bump event.
    pub fn publish_subject(
        &self,
        tenant_id: &str,
        user_id: &str,
        new_version: u64,
        reason: BumpReason,
    ) {
        let url = self.redis_url.clone();
        let secret = self.hmac_secret.clone();
        let tenant_id = tenant_id.to_string();
        let user_id = user_id.to_string();
        let reason_str = format!("{:?}", reason);

        may::go!(move || {
            let event = VersionBumpEvent::for_subject(&tenant_id, &user_id, new_version, reason);
            let json = match event.to_json() {
                Ok(j) => j,
                Err(e) => {
                    tracing::error!(error = %e, "failed to serialize event for publish_subject");
                    return;
                }
            };

            let message = match Self::sign_event(&json, &secret) {
                Ok(msg) => msg,
                Err(e) => {
                    tracing::error!(error = %e, "failed to sign event for publish_subject");
                    return;
                }
            };

            if let Err(e) = Self::publish_event(&url, &message) {
                tracing::error!(
                    tenant_id,
                    user_id,
                    new_version,
                    reason = reason_str,
                    error = %e,
                    "failed to publish subject version bump event",
                );
            }
        });
    }

    /// Sign an event JSON payload with HMAC-SHA256.
    fn sign_event(json: &str, secret: &[u8]) -> Result<String, String> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let mut mac =
            HmacSha256::new_from_slice(secret).map_err(|e| format!("invalid HMAC secret: {e}"))?;
        mac.update(json.as_bytes());
        let sig = hex::encode(mac.finalize().into_bytes());

        Ok(format!("{json}|{sig}"))
    }

    /// Publish a pre-signed message to Redis PUBLISH.
    fn publish_event(url: &str, message: &str) -> Result<(), String> {
        let mut conn = redis::Client::open(url)
            .map_err(|e| format!("failed to open Redis client: {e}"))?
            .get_connection()
            .map_err(|e| format!("failed to get Redis connection: {e}"))?;

        let _: i64 = conn
            .publish(VERSION_BUMP_CHANNEL, message)
            .map_err(|e| format!("failed to PUBLISH to Redis: {e}"))?;

        tracing::debug!(
            channel = VERSION_BUMP_CHANNEL,
            "published version bump event",
        );

        Ok(())
    }

    /// Get the HMAC secret for testing.
    #[doc(hidden)]
    pub fn hmac_secret(&self) -> &[u8] {
        &self.hmac_secret
    }
}

/// Create a `PublisherWrapper` from Redis config.
/// Returns `None` if Redis is not configured.
pub fn create_publisher(config: &sesame_common::config::AppConfig) -> Option<PublisherWrapper> {
    let redis = config.redis.as_ref()?;
    let url = redis.url.as_ref()?;
    let secret = redis.hmac_secret.as_ref()?;
    Some(PublisherWrapper::new(url, secret.clone().into_bytes()))
}
