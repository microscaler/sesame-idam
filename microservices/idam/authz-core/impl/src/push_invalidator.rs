//! Push invalidation publisher wrapper for authz-core.
//!
//! Provides a thread-safe, shared publisher that authz-changing controllers
//! call to emit version bump events to Redis pub/sub.
//!
//! # Usage
//!
//! Controllers call `publish_tenant()` or `publish_subject()` after performing
//! authz state changes. These are fire-and-forget — they spawn an async task
//! and return immediately.

use std::sync::Arc;

use sesame_token_versioning::VersionBumpPublisher;

/// Shared publisher handle wrapped for sync callers.
pub struct PublisherWrapper {
    redis_url: String,
    hmac_secret: Vec<u8>,
}

impl PublisherWrapper {
    /// Create a new publisher wrapper from a version bump publisher.
    /// Extracts the Redis URL from the publisher config for later use.
    pub fn new(redis_url: &str, hmac_secret: Vec<u8>) -> Self {
        Self {
            redis_url: redis_url.to_string(),
            hmac_secret,
        }
    }

    /// Fire-and-forget: publish a tenant-wide version bump event.
    ///
    /// This spawns an async task that publishes the event. The controller
    /// does not wait for the publish to complete — it is fire-and-forget.
    pub fn publish_tenant(
        &self,
        tenant_id: &str,
        new_version: u64,
        reason: sesame_token_versioning::BumpReason,
    ) {
        let url = self.redis_url.clone();
        let secret = self.hmac_secret.clone();

        tokio::task::spawn(async move {
            let publisher = match VersionBumpPublisher::new(&url, secret.clone()) {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!(error = %e, "failed to create publisher for publish_tenant");
                    return;
                }
            };
            if let Err(e) = publisher.publish_tenant(tenant_id, new_version, reason).await {
                tracing::error!(
                    tenant_id,
                    new_version,
                    reason = ?reason,
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
        reason: sesame_token_versioning::BumpReason,
    ) {
        let url = self.redis_url.clone();
        let secret = self.hmac_secret.clone();

        tokio::task::spawn(async move {
            let publisher = match VersionBumpPublisher::new(&url, secret.clone()) {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!(error = %e, "failed to create publisher for publish_subject");
                    return;
                }
            };
            if let Err(e) = publisher
                .publish_subject(tenant_id, user_id, new_version, reason)
                .await
            {
                tracing::error!(
                    tenant_id,
                    user_id,
                    new_version,
                    reason = ?reason,
                    error = %e,
                    "failed to publish subject version bump event",
                );
            }
        });
    }

    /// Get the HMAC secret for testing.
    #[doc(hidden)]
    pub fn hmac_secret(&self) -> &[u8] {
        &self.hmac_secret
    }
}

/// Create a `PublisherWrapper` from Redis config.
/// Returns `None` if Redis is not configured.
pub fn create_publisher(
    config: &crate::config::AppConfig,
) -> Option<PublisherWrapper> {
    let redis = config.redis.as_ref()?;
    let url = redis.url.as_ref()?;
    let secret = redis.hmac_secret.as_ref()?;
    Some(PublisherWrapper::new(url, secret.clone().into_bytes()))
}
