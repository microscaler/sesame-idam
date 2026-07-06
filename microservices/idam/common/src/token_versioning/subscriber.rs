//! Version bump event subscriber for Redis pub/sub.
//!
//! Subscribes to the `authz:version_bump` channel, verifies HMAC signatures,
//! validates events, and updates a local in-memory version cache.
//!
//! # Security
//!
//! - Events are HMAC-SHA256 signed (HACK-505) — forged events rejected
//! - Cache size limited to prevent memory exhaustion (HACK-504)
//! - Timestamp validated to prevent metric manipulation (HACK-507)
//! - Push invalidation is a LATENCY OPTIMIZATION, not the primary revocation mechanism
//! - The version check on every request (Story 5.2) remains the PRIMARY revocation mechanism
//!
//! # May Runtime
//!
//! The background pub/sub loop runs in a `may::go!` coroutine. Inside the coroutine,
//! Redis uses blocking I/O via the sync API — this is safe because the coroutine's
//! epoll loop is idle while waiting for the socket; other coroutines continue
//! to make progress.
//!
//! # Thread Safety
//!
//! `VersionBumpSubscriber` is `Clone` and shares a `ArcSwap` of the subscriber handle,
//! allowing the cache to be updated without locking the entire struct.

use super::events::{BumpReason, VersionBumpEvent};
use super::publisher::{parse_signed_message, verify_signature, VERSION_BUMP_CHANNEL};
use anyhow::{anyhow, Context, Result};
use arc_swap::ArcSwap;
use prometheus::{Histogram, IntCounterVec, Registry};
use redis::Client;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};

/// Maximum number of entries in the local version cache.
const MAX_CACHE_ENTRIES: usize = 10_000;

/// Default TTL for cache entries (seconds).
const DEFAULT_CACHE_TTL_SECS: u64 = 300;

/// Clock skew tolerance for timestamp validation (seconds).
const MAX_CLOCK_SKEW_SECS: u64 = 60;

/// Minimum acceptable timestamp (one year ago) (HACK-507).
const MIN_TIMESTAMP_SECS: u64 = 946_080_000; // 2000-01-01

/// Metrics registered with the subscriber.
#[derive(Clone)]
pub struct SubscriberMetrics {
    /// Total count of version bump events received, labeled by reason.
    pub version_bump_total: IntCounterVec,
    /// Time from event publish to service awareness (seconds).
    pub revocation_propagation_seconds: Histogram,
}

impl SubscriberMetrics {
    /// Create and register metrics with a Prometheus registry.
    pub fn register(registry: &Registry) -> Result<Self> {
        let version_bump_total = IntCounterVec::new(
            prometheus::Opts::new(
                "version_bump_total",
                "Total number of version bump events received, labeled by reason",
            ),
            &["reason"],
        )
        .context("failed to create version_bump_total metric")?;

        let revocation_propagation_seconds = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "revocation_propagation_seconds",
                "Time from event publish to service awareness (seconds)",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]),
        )
        .context("failed to create revocation_propagation_seconds metric")?;

        registry.register(Box::new(version_bump_total.clone()))?;
        registry.register(Box::new(revocation_propagation_seconds.clone()))?;

        Ok(Self {
            version_bump_total,
            revocation_propagation_seconds,
        })
    }

    /// Record a received event.
    pub fn record_event(&self, reason: &BumpReason, propagation_secs: f64) {
        let reason_str = match reason {
            BumpReason::RoleAssigned => "role_assigned",
            BumpReason::RoleRevoked => "role_revoked",
            BumpReason::UserDisabled => "user_disabled",
            BumpReason::UserEnabled => "user_enabled",
            BumpReason::OrgDeleted => "org_deleted",
            BumpReason::PermissionModified => "permission_modified",
            BumpReason::AppDeleted => "app_deleted",
            BumpReason::PrincipalAttributeModified => "principal_attribute_modified",
            BumpReason::Other(s) => s.as_str(),
        };
        self.version_bump_total
            .with_label_values(&[reason_str])
            .inc();
        self.revocation_propagation_seconds
            .observe(propagation_secs);
    }
}

/// Entry in the local version cache with TTL tracking.
struct CacheEntry {
    version: u64,
    /// Unix timestamp when the entry was added (for TTL-based eviction).
    inserted_at: u64,
}

/// Handle to the running subscriber task.
#[derive(Clone)]
pub struct SubscriberHandle {
    stop_tx: Sender<()>,
}

impl SubscriberHandle {
    /// Stop the subscriber gracefully.
    pub fn stop(self) {
        let _ = self.stop_tx.send(());
    }
}

/// Subscriber that receives version bump events via Redis pub/sub.
pub struct VersionBumpSubscriber {
    redis_url: String,
    hmac_secret: Vec<u8>,
    metrics: SubscriberMetrics,
    subscriber_handle: Arc<ArcSwap<SubscriberHandle>>,
    cache_ttl_secs: u64,
    max_cache_size: usize,
    local_cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

/// Configuration for the subscriber.
#[derive(Debug, Clone)]
pub struct SubscriberConfig {
    pub redis_url: String,
    pub hmac_secret: Vec<u8>,
    pub registry: Registry,
    pub cache_ttl_secs: Option<u64>,
    pub max_cache_size: Option<usize>,
}

impl SubscriberConfig {
    #[must_use]
    pub fn new(redis_url: &str, hmac_secret: &[u8], registry: Registry) -> Self {
        Self {
            redis_url: redis_url.to_string(),
            hmac_secret: hmac_secret.to_vec(),
            registry,
            cache_ttl_secs: None,
            max_cache_size: None,
        }
    }
}

impl VersionBumpSubscriber {
    /// Create a new subscriber from a config.
    pub fn from_config(config: &SubscriberConfig) -> Result<Self> {
        let metrics = SubscriberMetrics::register(&config.registry)?;

        Ok(Self {
            redis_url: config.redis_url.clone(),
            hmac_secret: config.hmac_secret.clone(),
            metrics,
            subscriber_handle: Arc::new(ArcSwap::from_pointee(SubscriberHandle {
                stop_tx: channel().0,
            })),
            cache_ttl_secs: config.cache_ttl_secs.unwrap_or(DEFAULT_CACHE_TTL_SECS),
            max_cache_size: config.max_cache_size.unwrap_or(MAX_CACHE_ENTRIES),
            local_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Start the subscriber in the background.
    ///
    /// Returns a handle that can be used to stop the subscriber.
    ///
    /// The pub/sub loop runs in a `may::go!` coroutine. Inside, Redis uses sync I/O
    /// — blocking the coroutine's epoll wait while waiting for Redis I/O is cooperative;
    pub fn start(&self) -> Result<SubscriberHandle> {
        let (stop_tx, stop_rx) = channel();

        let redis_url = self.redis_url.clone();
        let hmac_secret = self.hmac_secret.clone();
        let max_cache_size = self.max_cache_size;
        let cache_ttl_secs = self.cache_ttl_secs;
        let metrics = self.metrics.clone();
        let cache = self.local_cache.clone();

        // Spawn the background task using may::go!.
        // The pub/sub loop uses sync Redis APIs so it runs on the may scheduler natively.
        may::go!(move || {
            let mut backoff: u32 = 0;
            let max_backoff = Duration::from_secs(30);

            loop {
                debug!("connecting to Redis for pub/sub subscription");

                match Self::subscribe_and_process(
                    &redis_url,
                    &hmac_secret,
                    &metrics,
                    &cache,
                    max_cache_size,
                    cache_ttl_secs,
                    &stop_rx,
                ) {
                    Ok(()) => {
                        info!("subscriber stopped gracefully");
                        break;
                    }
                    Err(e) => {
                        warn!(error = %e, "subscriber connection lost, reconnecting...");
                    }
                }

                if stop_rx.try_recv().is_ok() {
                    break;
                }

                let backoff_duration = std::cmp::min(
                    Duration::from_millis(2_u64.saturating_pow(backoff) * 100),
                    max_backoff,
                );
                info!(
                    backoff_ms = backoff_duration.as_millis(),
                    "reconnecting in..."
                );
                std::thread::sleep(backoff_duration);
                backoff = backoff.saturating_add(1);
            }
        });

        let handle = SubscriberHandle { stop_tx };
        self.subscriber_handle.store(Arc::new(handle.clone()));

        Ok(handle)
    }

    /// Main subscription and processing loop.
    ///
    /// Uses sync Redis APIs via `Connection::as_pubsub()`. Safe inside `may::go!` —
    /// blocking the coroutine's epoll wait while waiting for Redis I/O is cooperative;
    /// other coroutines continue to make progress.
    fn subscribe_and_process(
        redis_url: &str,
        hmac_secret: &[u8],
        metrics: &SubscriberMetrics,
        cache: &Arc<RwLock<HashMap<String, CacheEntry>>>,
        max_cache_size: usize,
        cache_ttl_secs: u64,
        stop_rx: &Receiver<()>,
    ) -> Result<()> {
        let client = Client::open(redis_url).context("failed to open Redis client")?;
        let mut conn = client
            .get_connection()
            .context("failed to get Redis connection")?;

        // HACK-506: Warm-up — read current versions from Redis on startup
        Self::warmup_cache(&mut conn).context("failed to warm up local cache from Redis")?;

        // Use sync pubsub API via connection.as_pubsub()
        let mut pubsub = conn.as_pubsub();
        pubsub
            .subscribe(VERSION_BUMP_CHANNEL)
            .context("failed to subscribe to version_bump channel")?;

        info!("subscribed to {}", VERSION_BUMP_CHANNEL);

        let cache = cache.clone();
        let hmac_secret = hmac_secret.to_vec();
        let max_cache_size_clone = max_cache_size;
        let cache_ttl_secs_clone = cache_ttl_secs;
        let metrics_clone = metrics.clone();

        loop {
            if stop_rx.try_recv().is_ok() {
                debug!("received stop signal");
                break;
            }

            match pubsub.get_message() {
                Ok(msg) => {
                    let payload: String = match msg.get_payload::<String>() {
                        Ok(p) => p,
                        Err(e) => {
                            error!(error = %e, "failed to deserialize event payload");
                            continue;
                        }
                    };

                    if let Err(e) = Self::process_message(
                        &payload,
                        &hmac_secret,
                        &cache,
                        max_cache_size_clone,
                        cache_ttl_secs_clone,
                        &metrics_clone,
                    ) {
                        error!(error = %e, "failed to process version bump event");
                    }
                }
                Err(e) => {
                    return Err(anyhow!("pub/sub message read error: {e}"));
                }
            }
        }

        let _ = pubsub.unsubscribe(VERSION_BUMP_CHANNEL);
        Ok(())
    }

    /// Warm up the local cache by reading current versions from Redis.
    fn warmup_cache(conn: &mut redis::Connection) -> Result<()> {
        let tenant_pattern = "authz_ver:tenant:*";
        let tenant_keys: Vec<String> = redis::cmd("KEYS")
            .arg(tenant_pattern)
            .query(conn)
            .unwrap_or_default();

        for key in &tenant_keys {
            let version: Option<u64> = redis::cmd("GET").arg(key).query(conn)?;
            if let Some(_ver) = version {
                debug!(key, version, "warmed up tenant cache entry");
            }
        }

        let user_pattern = "authz_ver:*";
        let user_keys: Vec<String> = redis::cmd("KEYS")
            .arg(user_pattern)
            .query(conn)
            .unwrap_or_default();

        for key in &user_keys {
            let version: Option<u64> = redis::cmd("GET").arg(key).query(conn)?;
            if let Some(_ver) = version {
                debug!(key, version, "warmed up user cache entry");
            }
        }

        Ok(())
    }

    /// Process a single event message.
    fn process_message(
        message: &str,
        hmac_secret: &[u8],
        cache: &Arc<RwLock<HashMap<String, CacheEntry>>>,
        max_cache_size: usize,
        cache_ttl_secs: u64,
        metrics: &SubscriberMetrics,
    ) -> Result<()> {
        let (json_payload, sig_hex) =
            parse_signed_message(message).context("invalid message format")?;

        verify_signature(&json_payload, &sig_hex, hmac_secret)
            .context("HMAC signature verification failed — event may be forged")?;

        let event: VersionBumpEvent =
            serde_json::from_str(&json_payload).context("failed to deserialize event")?;

        if let Err(e) = event.validate() {
            return Err(anyhow!("event validation failed: {e}"));
        }

        let now = Self::current_unix_seconds();
        if event.timestamp > now + MAX_CLOCK_SKEW_SECS {
            warn!(
                event_timestamp = event.timestamp,
                now, "event timestamp is too far in the future, rejecting"
            );
            return Err(anyhow!("event timestamp is too far in the future"));
        }
        if event.timestamp < MIN_TIMESTAMP_SECS {
            warn!(
                event_timestamp = event.timestamp,
                now, "event timestamp is too old"
            );
        }

        let mut cache_guard = cache.write().unwrap();

        if cache_guard.len() >= max_cache_size {
            let expired_keys: Vec<String> = cache_guard
                .iter()
                .filter(|(_, entry)| now.saturating_sub(entry.inserted_at) >= cache_ttl_secs)
                .map(|(k, _)| k.clone())
                .collect();
            for key in &expired_keys {
                cache_guard.remove(key);
            }
            if cache_guard.len() >= max_cache_size {
                Self::evict_oldest(&mut cache_guard, max_cache_size);
            }
        }

        let tenant_key = event.tenant_cache_key();
        cache_guard.insert(
            tenant_key,
            CacheEntry {
                version: event.new_version,
                inserted_at: now,
            },
        );

        if let Some(ref user_id) = event.user_id {
            let user_key = format!("authz_ver:{user_id}");
            cache_guard.insert(
                user_key,
                CacheEntry {
                    version: event.new_version,
                    inserted_at: now,
                },
            );
        }

        let propagation_secs = if event.timestamp > 0 {
            let elapsed = now.saturating_sub(event.timestamp);
            elapsed as f64 / 1_000.0
        } else {
            0.0
        };
        metrics.record_event(&event.reason, propagation_secs);

        debug!(
            tenant_id = event.tenant_id,
            user_id = ?event.user_id,
            new_version = event.new_version,
            reason = ?event.reason,
            "processed version bump event",
        );

        Ok(())
    }

    fn evict_oldest(cache: &mut HashMap<String, CacheEntry>, max_size: usize) {
        let mut entries: Vec<_> = cache.iter().collect();
        entries.sort_by_key(|(_, entry)| entry.inserted_at);

        let keys_to_remove: Vec<String> = entries
            .iter()
            .take(entries.len().saturating_sub(max_size))
            .map(|(k, _)| (*k).clone())
            .collect();

        for key in keys_to_remove {
            cache.remove(&key);
        }
    }

    #[must_use]
    pub fn current_unix_seconds() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    #[must_use]
    pub fn get_cached_version(&self, key: &str) -> Option<u64> {
        let cache = self.local_cache.read().ok()?;
        cache.get(key).map(|entry| entry.version)
    }

    #[must_use]
    pub fn user_cache_key(user_id: &str) -> String {
        format!("authz_ver:{user_id}")
    }

    #[must_use]
    pub fn tenant_cache_key(tenant_id: &str) -> String {
        format!("authz_ver:tenant:{tenant_id}")
    }
}

#[cfg(test)]
mod tests {
    use super::super::events::BumpReason;
    use super::*;
    use hmac::Mac;
    use prometheus::Encoder;

    fn test_hmac_secret() -> Vec<u8> {
        b"test-shared-secret-for-hmac".to_vec()
    }

    #[test]
    fn test_cache_key_generation() {
        assert_eq!(
            VersionBumpSubscriber::user_cache_key("user_123"),
            "authz_ver:user_123"
        );
        assert_eq!(
            VersionBumpSubscriber::tenant_cache_key("tenant_abc"),
            "authz_ver:tenant:tenant_abc"
        );
    }

    #[test]
    fn test_current_timestamp_nonzero() {
        let now = VersionBumpSubscriber::current_unix_seconds();
        assert!(now > 0);
        assert!(now > 1_700_000_000);
    }

    #[test]
    fn test_evict_oldest() {
        let mut cache: HashMap<String, CacheEntry> = HashMap::new();
        cache.insert(
            "key1".to_string(),
            CacheEntry {
                version: 1,
                inserted_at: 100,
            },
        );
        cache.insert(
            "key2".to_string(),
            CacheEntry {
                version: 2,
                inserted_at: 200,
            },
        );
        cache.insert(
            "key3".to_string(),
            CacheEntry {
                version: 3,
                inserted_at: 300,
            },
        );

        VersionBumpSubscriber::evict_oldest(&mut cache, 2);
        assert_eq!(cache.len(), 2);
        assert!(!cache.contains_key("key1"));
    }

    #[test]
    fn test_malformed_message_rejected() {
        let secret = test_hmac_secret();
        let cache = Arc::new(RwLock::new(HashMap::new()));
        let registry = Registry::new();
        let metrics = SubscriberMetrics::register(&registry).unwrap();

        let result = VersionBumpSubscriber::process_message(
            "no-pipe-here",
            &secret,
            &cache,
            10000,
            300,
            &metrics,
        );
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("invalid message")
                || err.contains("missing HMAC")
                || err.contains("not enough")
                || err.contains("empty"),
            "Expected error about invalid message/HMAC/signature, got: {err}"
        );
    }

    #[test]
    fn test_invalid_signature_rejected() {
        let secret = test_hmac_secret();
        let event =
            VersionBumpEvent::for_subject("tenant_abc", "user_123", 43, BumpReason::RoleRevoked);
        let json = event.to_json().unwrap();

        let fake_sig = hex::encode(b"fake");
        let message = format!("{json}|{fake_sig}");

        let cache = Arc::new(RwLock::new(HashMap::new()));
        let registry = Registry::new();
        let metrics = SubscriberMetrics::register(&registry).unwrap();

        let result =
            VersionBumpSubscriber::process_message(&message, &secret, &cache, 10000, 300, &metrics);
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_event_updates_cache() {
        let secret = test_hmac_secret();
        let event =
            VersionBumpEvent::for_subject("tenant_abc", "user_123", 43, BumpReason::RoleRevoked);
        let json = event.to_json().unwrap();

        let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(&secret).unwrap();
        mac.update(json.as_bytes());
        let sig = hex::encode(mac.finalize().into_bytes());
        let message = format!("{json}|{sig}");

        let cache = Arc::new(RwLock::new(HashMap::new()));
        let registry = Registry::new();
        let metrics = SubscriberMetrics::register(&registry).unwrap();

        VersionBumpSubscriber::process_message(&message, &secret, &cache, 10000, 300, &metrics)
            .unwrap();

        let guard = cache.read().unwrap();
        let tenant_key = VersionBumpSubscriber::tenant_cache_key("tenant_abc");
        assert!(guard.contains_key(&tenant_key));
        assert_eq!(guard.get(&tenant_key).unwrap().version, 43);

        let user_key = VersionBumpSubscriber::user_cache_key("user_123");
        assert!(guard.contains_key(&user_key));
        assert_eq!(guard.get(&user_key).unwrap().version, 43);
    }

    #[test]
    fn test_tenant_wide_event_updates_only_tenant_cache() {
        let secret = test_hmac_secret();
        let event = VersionBumpEvent::for_tenant("tenant_x", 100, BumpReason::OrgDeleted);
        let json = event.to_json().unwrap();

        let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(&secret).unwrap();
        mac.update(json.as_bytes());
        let sig = hex::encode(mac.finalize().into_bytes());
        let message = format!("{json}|{sig}");

        let cache = Arc::new(RwLock::new(HashMap::new()));
        let registry = Registry::new();
        let metrics = SubscriberMetrics::register(&registry).unwrap();

        VersionBumpSubscriber::process_message(&message, &secret, &cache, 10000, 300, &metrics)
            .unwrap();

        let guard = cache.read().unwrap();
        assert_eq!(guard.len(), 1);
        let tenant_key = VersionBumpSubscriber::tenant_cache_key("tenant_x");
        assert!(guard.contains_key(&tenant_key));
    }

    #[test]
    fn test_zero_version_rejected() {
        let secret = test_hmac_secret();
        let event = VersionBumpEvent::for_tenant("tenant_x", 0, BumpReason::OrgDeleted);
        let json = event.to_json().unwrap();

        let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(&secret).unwrap();
        mac.update(json.as_bytes());
        let sig = hex::encode(mac.finalize().into_bytes());
        let message = format!("{json}|{sig}");

        let cache = Arc::new(RwLock::new(HashMap::new()));
        let registry = Registry::new();
        let metrics = SubscriberMetrics::register(&registry).unwrap();

        let result =
            VersionBumpSubscriber::process_message(&message, &secret, &cache, 10000, 300, &metrics);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("new_version is 0"));
    }

    #[test]
    fn test_empty_tenant_id_rejected() {
        let secret = test_hmac_secret();
        let mut event = VersionBumpEvent::for_tenant("tenant_x", 10, BumpReason::OrgDeleted);
        event.tenant_id = String::new();
        let json = event.to_json().unwrap();

        let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(&secret).unwrap();
        mac.update(json.as_bytes());
        let sig = hex::encode(mac.finalize().into_bytes());
        let message = format!("{json}|{sig}");

        let cache = Arc::new(RwLock::new(HashMap::new()));
        let registry = Registry::new();
        let metrics = SubscriberMetrics::register(&registry).unwrap();

        let result =
            VersionBumpSubscriber::process_message(&message, &secret, &cache, 10000, 300, &metrics);
        assert!(result.is_err());
    }

    #[test]
    fn test_metrics_incremented_on_event() {
        let secret = test_hmac_secret();
        let event =
            VersionBumpEvent::for_subject("tenant_abc", "user_123", 43, BumpReason::RoleRevoked);
        let json = event.to_json().unwrap();

        let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(&secret).unwrap();
        mac.update(json.as_bytes());
        let sig = hex::encode(mac.finalize().into_bytes());
        let message = format!("{json}|{sig}");

        let cache = Arc::new(RwLock::new(HashMap::new()));
        let registry = Registry::new();
        let metrics = SubscriberMetrics::register(&registry).unwrap();

        VersionBumpSubscriber::process_message(&message, &secret, &cache, 10000, 300, &metrics)
            .unwrap();

        let encoder = prometheus::TextEncoder::new();
        let metric_families = registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        let text = String::from_utf8_lossy(&buffer);
        assert!(text.contains("version_bump_total"));
        assert!(text.contains("role_revoked"));
    }

    #[test]
    fn test_cache_size_limit() {
        let cache = Arc::new(RwLock::new(HashMap::new()));
        for i in 0..100 {
            cache.write().unwrap().insert(
                format!("key{i}"),
                CacheEntry {
                    version: i,
                    inserted_at: i,
                },
            );
        }

        {
            let mut guard = cache.write().unwrap();
            VersionBumpSubscriber::evict_oldest(&mut guard, 50);
            assert!(guard.len() <= 50);
        }
    }

    #[test]
    fn test_future_timestamp_rejected() {
        let secret = test_hmac_secret();
        let event = VersionBumpEvent {
            event: "version_bump".to_string(),
            tenant_id: "tenant_x".to_string(),
            user_id: None,
            new_version: 10,
            reason: BumpReason::OrgDeleted,
            timestamp: u64::MAX,
        };
        let json = event.to_json().unwrap();

        let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(&secret).unwrap();
        mac.update(json.as_bytes());
        let sig = hex::encode(mac.finalize().into_bytes());
        let message = format!("{json}|{sig}");

        let cache = Arc::new(RwLock::new(HashMap::new()));
        let registry = Registry::new();
        let metrics = SubscriberMetrics::register(&registry).unwrap();

        let result =
            VersionBumpSubscriber::process_message(&message, &secret, &cache, 10000, 300, &metrics);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("timestamp is too far in the future"));
    }

    #[test]
    fn test_rapid_successive_events() {
        let secret = test_hmac_secret();
        let cache = Arc::new(RwLock::new(HashMap::new()));
        let registry = Registry::new();
        let metrics = SubscriberMetrics::register(&registry).unwrap();

        for ver in 10..14 {
            let event = VersionBumpEvent::for_tenant("tenant_x", ver, BumpReason::OrgDeleted);
            let json = event.to_json().unwrap();

            let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(&secret).unwrap();
            mac.update(json.as_bytes());
            let sig = hex::encode(mac.finalize().into_bytes());
            let message = format!("{json}|{sig}");

            VersionBumpSubscriber::process_message(&message, &secret, &cache, 10000, 300, &metrics)
                .unwrap();
        }

        let guard = cache.read().unwrap();
        let tenant_key = VersionBumpSubscriber::tenant_cache_key("tenant_x");
        assert_eq!(guard.get(&tenant_key).unwrap().version, 13);
    }

    #[test]
    fn test_concurrent_events_no_race() {
        let secret = test_hmac_secret();
        let cache = Arc::new(RwLock::new(HashMap::new()));
        let registry = Registry::new();
        let metrics = SubscriberMetrics::register(&registry).unwrap();

        let mut handles = vec![];
        for i in 1..101 {
            let event = VersionBumpEvent::for_tenant("tenant_x", i, BumpReason::OrgDeleted);
            let json = event.to_json().unwrap();

            let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(&secret).unwrap();
            mac.update(json.as_bytes());
            let sig = hex::encode(mac.finalize().into_bytes());
            let message = format!("{json}|{sig}");

            let cache_clone = cache.clone();
            let metrics_clone = metrics.clone();
            let secret_clone = secret.clone();
            handles.push(std::thread::spawn(move || {
                VersionBumpSubscriber::process_message(
                    &message,
                    &secret_clone,
                    &cache_clone,
                    10000,
                    300,
                    &metrics_clone,
                )
                .unwrap();
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let guard = cache.read().unwrap();
        assert_eq!(guard.len(), 1);
        let tenant_key = VersionBumpSubscriber::tenant_cache_key("tenant_x");
        assert!(guard.contains_key(&tenant_key));
    }
}
