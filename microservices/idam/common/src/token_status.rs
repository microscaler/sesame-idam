//! Redis-backed dynamic JWT status checks for BRRTRouter.
//!
//! Signature and standard claims are validated by `JwksBearerProvider`. This module supplies
//! the Sesame-specific read side: targeted `jti` denylisting plus subject/tenant version checks.
//! Redis errors fail closed. Only monotonic rejection results are cached: an active token is
//! checked against Redis on every protected request, so logout cannot be hidden by a negative
//! cache entry.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use brrtrouter::security::{JwtTokenStatus, JwtTokenStatusChecker};
use dashmap::DashMap;
use redis::Client;
use serde_json::Value;

use crate::token_versioning::{subject_key, tenant_key};

const DEFAULT_MAX_CACHE_ENTRIES: usize = 10_000;
const DEFAULT_REDIS_SHARDS: u64 = 16;
const DEFAULT_REDIS_TIMEOUT: Duration = Duration::from_millis(250);

static ACTIVE_TOTAL: AtomicU64 = AtomicU64::new(0);
static DENYLIST_TOTAL: AtomicU64 = AtomicU64::new(0);
static VERSION_TOTAL: AtomicU64 = AtomicU64::new(0);
static DEPENDENCY_TOTAL: AtomicU64 = AtomicU64::new(0);
static INVALID_TOTAL: AtomicU64 = AtomicU64::new(0);

fn observe(status: JwtTokenStatus) -> JwtTokenStatus {
    let counter = match status {
        JwtTokenStatus::Active => &ACTIVE_TOTAL,
        JwtTokenStatus::Revoked => &DENYLIST_TOTAL,
        JwtTokenStatus::Stale => &VERSION_TOTAL,
        JwtTokenStatus::Unavailable => &DEPENDENCY_TOTAL,
        JwtTokenStatus::Invalid => &INVALID_TOTAL,
    };
    counter.fetch_add(1, Ordering::Relaxed);
    status
}

/// Render token-status counters for BRRTRouter's Prometheus scrape extension.
///
/// Labels are a fixed low-cardinality set and never contain token or claim values.
#[must_use]
pub fn token_status_prometheus_scrape_text() -> String {
    format!(
        concat!(
            "# HELP sesame_token_status_checks_total Dynamic JWT status decisions.\n",
            "# TYPE sesame_token_status_checks_total counter\n",
            "sesame_token_status_checks_total{{result=\"active\"}} {}\n",
            "sesame_token_status_checks_total{{result=\"denylist\"}} {}\n",
            "sesame_token_status_checks_total{{result=\"version\"}} {}\n",
            "sesame_token_status_checks_total{{result=\"dependency\"}} {}\n",
            "sesame_token_status_checks_total{{result=\"invalid\"}} {}\n"
        ),
        ACTIVE_TOTAL.load(Ordering::Relaxed),
        DENYLIST_TOTAL.load(Ordering::Relaxed),
        VERSION_TOTAL.load(Ordering::Relaxed),
        DEPENDENCY_TOTAL.load(Ordering::Relaxed),
        INVALID_TOTAL.load(Ordering::Relaxed),
    )
}

#[derive(Debug, Clone, Copy)]
struct AuthoritativeStatus {
    revoked: bool,
    subject_version: u64,
    tenant_version: u64,
}

trait TokenStatusLookup: Send + Sync {
    fn lookup(
        &self,
        jti: &str,
        subject: &str,
        tenant_id: &str,
    ) -> Result<AuthoritativeStatus, String>;
}

struct RedisTokenStatusLookup {
    client: Client,
    connections: Vec<Mutex<Option<redis::Connection>>>,
    timeout: Duration,
}

impl RedisTokenStatusLookup {
    fn new(redis_url: &str, timeout: Duration) -> Result<Self, String> {
        let client = Client::open(redis_url)
            .map_err(|error| format!("invalid REDIS_URL for token status: {error}"))?;
        Ok(Self {
            client,
            connections: (0..DEFAULT_REDIS_SHARDS)
                .map(|_| Mutex::new(None))
                .collect(),
            timeout,
        })
    }

    fn shard(&self, jti: &str) -> &Mutex<Option<redis::Connection>> {
        let mut hasher = DefaultHasher::new();
        jti.hash(&mut hasher);
        let index = usize::try_from(hasher.finish() % DEFAULT_REDIS_SHARDS).unwrap_or_default();
        &self.connections[index]
    }

    fn connect(&self) -> Result<redis::Connection, String> {
        let connection = self
            .client
            .get_connection_with_timeout(self.timeout)
            .map_err(|error| format!("token-status Redis unavailable: {error}"))?;
        connection
            .set_read_timeout(Some(self.timeout))
            .map_err(|error| format!("token-status Redis read timeout setup failed: {error}"))?;
        connection
            .set_write_timeout(Some(self.timeout))
            .map_err(|error| format!("token-status Redis write timeout setup failed: {error}"))?;
        Ok(connection)
    }

    fn query(
        connection: &mut redis::Connection,
        jti: &str,
        subject: &str,
        tenant_id: &str,
    ) -> redis::RedisResult<AuthoritativeStatus> {
        let (revoked, subject_version, tenant_version): (bool, Option<u64>, Option<u64>) =
            redis::pipe()
                .cmd("EXISTS")
                .arg(format!("denylist:{jti}"))
                .cmd("GET")
                .arg(subject_key(subject))
                .cmd("GET")
                .arg(tenant_key(tenant_id))
                .query(connection)?;
        Ok(AuthoritativeStatus {
            revoked,
            subject_version: subject_version.unwrap_or(0),
            tenant_version: tenant_version.unwrap_or(0),
        })
    }
}

impl TokenStatusLookup for RedisTokenStatusLookup {
    fn lookup(
        &self,
        jti: &str,
        subject: &str,
        tenant_id: &str,
    ) -> Result<AuthoritativeStatus, String> {
        let mut guard = self
            .shard(jti)
            .lock()
            .map_err(|_| "token-status Redis connection lock poisoned".to_string())?;
        if guard.is_none() {
            *guard = Some(self.connect()?);
        }

        let result = Self::query(
            guard
                .as_mut()
                .ok_or_else(|| "token-status Redis connection unavailable".to_string())?,
            jti,
            subject,
            tenant_id,
        );
        if result.is_err() {
            // Drop the failed connection so the next request can reconnect.
            *guard = None;
        }
        result.map_err(|error| format!("token-status Redis query failed: {error}"))
    }
}

/// Sesame denylist and token-version checker used by every JWKS consumer.
pub struct SesameTokenStatusChecker {
    lookup: Arc<dyn TokenStatusLookup>,
    rejection_cache: DashMap<String, JwtTokenStatus>,
    max_cache_entries: usize,
}

impl SesameTokenStatusChecker {
    /// Construct from an explicit Redis URL.
    ///
    /// # Errors
    ///
    /// Returns an error when the Redis URL is invalid.
    pub fn from_redis_url(redis_url: &str) -> Result<Self, String> {
        Self::from_redis_url_with_timeout(redis_url, DEFAULT_REDIS_TIMEOUT)
    }

    /// Construct from an explicit Redis URL and I/O timeout.
    ///
    /// # Errors
    ///
    /// Returns an error when the Redis URL is invalid.
    pub fn from_redis_url_with_timeout(redis_url: &str, timeout: Duration) -> Result<Self, String> {
        if timeout.is_zero() {
            return Err("token-status Redis timeout must be greater than zero".to_string());
        }
        Ok(Self {
            lookup: Arc::new(RedisTokenStatusLookup::new(redis_url, timeout)?),
            rejection_cache: DashMap::new(),
            max_cache_entries: DEFAULT_MAX_CACHE_ENTRIES,
        })
    }

    /// Construct from the required `REDIS_URL` environment variable.
    ///
    /// # Errors
    ///
    /// Returns an error when `REDIS_URL` is absent or invalid.
    pub fn from_env() -> Result<Self, String> {
        let redis_url = std::env::var("REDIS_URL")
            .map_err(|_| "REDIS_URL is required for fail-closed token status checks".to_string())?;
        Self::from_redis_url(&redis_url)
    }

    #[cfg(test)]
    fn with_lookup(lookup: Arc<dyn TokenStatusLookup>) -> Self {
        Self {
            lookup,
            rejection_cache: DashMap::new(),
            max_cache_entries: DEFAULT_MAX_CACHE_ENTRIES,
        }
    }

    fn cached_rejection(&self, jti: &str) -> Option<JwtTokenStatus> {
        self.rejection_cache.get(jti).map(|entry| *entry)
    }

    fn cache_rejection(&self, jti: &str, status: JwtTokenStatus) {
        if !matches!(status, JwtTokenStatus::Revoked | JwtTokenStatus::Stale) {
            return;
        }
        if self.rejection_cache.len() >= self.max_cache_entries {
            if let Some(eviction_key) = self
                .rejection_cache
                .iter()
                .next()
                .map(|entry| entry.key().clone())
            {
                self.rejection_cache.remove(&eviction_key);
            }
        }
        self.rejection_cache.insert(jti.to_string(), status);
    }
}

impl JwtTokenStatusChecker for SesameTokenStatusChecker {
    fn check(&self, claims: &Value) -> JwtTokenStatus {
        let Some(jti) = claims
            .get("jti")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
        else {
            return observe(JwtTokenStatus::Invalid);
        };
        let Some(subject) = claims
            .get("sub")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
        else {
            return observe(JwtTokenStatus::Invalid);
        };
        let Some(tenant_id) = claims
            .get("tenant_id")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
        else {
            return observe(JwtTokenStatus::Invalid);
        };
        let Some(claimed_version) = claims.get("ver").and_then(Value::as_u64) else {
            return observe(JwtTokenStatus::Invalid);
        };

        if let Some(status) = self.cached_rejection(jti) {
            return observe(status);
        }

        let authoritative = match self.lookup.lookup(jti, subject, tenant_id) {
            Ok(status) => status,
            Err(error) => {
                tracing::error!(event = "token_status_unavailable", error = %error);
                return observe(JwtTokenStatus::Unavailable);
            }
        };
        let status = if authoritative.revoked {
            JwtTokenStatus::Revoked
        } else if claimed_version < authoritative.subject_version
            || claimed_version < authoritative.tenant_version
        {
            JwtTokenStatus::Stale
        } else {
            JwtTokenStatus::Active
        };
        self.cache_rejection(jti, status);
        observe(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FixedLookup {
        status: Result<AuthoritativeStatus, String>,
        calls: AtomicUsize,
    }

    impl TokenStatusLookup for FixedLookup {
        fn lookup(
            &self,
            _jti: &str,
            _subject: &str,
            _tenant_id: &str,
        ) -> Result<AuthoritativeStatus, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.status.clone()
        }
    }

    fn claims(version: u64) -> Value {
        serde_json::json!({
            "jti": "jti-1",
            "sub": "user-1",
            "tenant_id": "tenant-1",
            "ver": version,
        })
    }

    fn checker(
        status: Result<AuthoritativeStatus, String>,
    ) -> (SesameTokenStatusChecker, Arc<FixedLookup>) {
        let lookup = Arc::new(FixedLookup {
            status,
            calls: AtomicUsize::new(0),
        });
        let checker = SesameTokenStatusChecker::with_lookup(
            Arc::clone(&lookup) as Arc<dyn TokenStatusLookup>
        );
        (checker, lookup)
    }

    #[test]
    fn revoked_jti_is_rejected() {
        let (checker, _) = checker(Ok(AuthoritativeStatus {
            revoked: true,
            subject_version: 1,
            tenant_version: 1,
        }));
        assert_eq!(checker.check(&claims(1)), JwtTokenStatus::Revoked);
    }

    #[test]
    fn stale_subject_or_tenant_version_is_rejected() {
        let (subject_checker, _) = checker(Ok(AuthoritativeStatus {
            revoked: false,
            subject_version: 2,
            tenant_version: 1,
        }));
        assert_eq!(subject_checker.check(&claims(1)), JwtTokenStatus::Stale);

        let (tenant_checker, _) = checker(Ok(AuthoritativeStatus {
            revoked: false,
            subject_version: 1,
            tenant_version: 2,
        }));
        assert_eq!(tenant_checker.check(&claims(1)), JwtTokenStatus::Stale);
    }

    #[test]
    fn active_result_is_never_cached() {
        let (checker, lookup) = checker(Ok(AuthoritativeStatus {
            revoked: false,
            subject_version: 1,
            tenant_version: 1,
        }));
        assert_eq!(checker.check(&claims(1)), JwtTokenStatus::Active);
        assert_eq!(checker.check(&claims(1)), JwtTokenStatus::Active);
        assert_eq!(lookup.calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn known_revocation_is_cached_without_reacceptance() {
        let (checker, lookup) = checker(Ok(AuthoritativeStatus {
            revoked: true,
            subject_version: 1,
            tenant_version: 1,
        }));
        assert_eq!(checker.check(&claims(1)), JwtTokenStatus::Revoked);
        assert_eq!(checker.check(&claims(1)), JwtTokenStatus::Revoked);
        assert_eq!(lookup.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn prometheus_metrics_use_only_fixed_result_labels() {
        let metrics = token_status_prometheus_scrape_text();
        for result in ["active", "denylist", "version", "dependency", "invalid"] {
            assert!(metrics.contains(&format!("result=\"{result}\"")));
        }
        assert!(!metrics.contains("jti-1"));
        assert!(!metrics.contains("user-1"));
        assert!(!metrics.contains("tenant-1"));
    }

    #[test]
    fn dependency_failure_and_missing_claims_fail_closed() {
        let (checker, _) = checker(Err("redis unavailable".to_string()));
        assert_eq!(checker.check(&claims(1)), JwtTokenStatus::Unavailable);
        assert_eq!(
            checker.check(&serde_json::json!({})),
            JwtTokenStatus::Invalid
        );
    }

    #[test]
    fn unreachable_redis_fails_closed_within_the_configured_bound() {
        let checker = SesameTokenStatusChecker::from_redis_url_with_timeout(
            "redis://127.0.0.1:1",
            Duration::from_millis(50),
        )
        .expect("a syntactically valid Redis URL should initialize");
        let started = std::time::Instant::now();

        assert_eq!(checker.check(&claims(1)), JwtTokenStatus::Unavailable);
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "fail-closed Redis rejection exceeded its safety bound"
        );
    }

    #[test]
    fn zero_redis_timeout_is_rejected_at_startup() {
        assert!(SesameTokenStatusChecker::from_redis_url_with_timeout(
            "redis://127.0.0.1:6379",
            Duration::ZERO,
        )
        .is_err());
    }

    #[test]
    fn redis_enforces_denylist_and_version_changes() {
        use redis::Commands;

        let redis_url = std::env::var("TEST_REDIS_URL").or_else(|_| std::env::var("REDIS_URL"));
        let Ok(redis_url) = redis_url else {
            eprintln!("skipping Redis integration test: TEST_REDIS_URL/REDIS_URL is not set");
            return;
        };

        let suffix = uuid::Uuid::new_v4();
        let jti = format!("p0-jti-{suffix}");
        let subject = format!("p0-subject-{suffix}");
        let tenant = format!("p0-tenant-{suffix}");
        let denylist_key = format!("denylist:{jti}");
        let subject_version_key = subject_key(&subject);
        let tenant_version_key = tenant_key(&tenant);
        let test_claims = serde_json::json!({
            "jti": jti,
            "sub": subject,
            "tenant_id": tenant,
            "ver": 1,
        });

        let client = redis::Client::open(redis_url.as_str()).expect("valid Redis URL");
        let Ok(mut connection) = client.get_connection() else {
            eprintln!("skipping Redis integration test: configured Redis is not reachable");
            return;
        };
        let checker = SesameTokenStatusChecker::from_redis_url(&redis_url)
            .expect("token-status checker should initialize");

        let _: usize = connection
            .del((&denylist_key, &subject_version_key, &tenant_version_key))
            .expect("clear integration-test keys");
        assert_eq!(checker.check(&test_claims), JwtTokenStatus::Active);

        let _: () = connection
            .set_ex(&denylist_key, "1", 60)
            .expect("write denylist entry");
        assert_eq!(checker.check(&test_claims), JwtTokenStatus::Revoked);

        let _: usize = connection.del(&denylist_key).expect("clear denylist entry");
        let _: () = connection
            .set_ex(&subject_version_key, 2_u64, 60)
            .expect("write subject version");
        let version_checker = SesameTokenStatusChecker::from_redis_url(&redis_url)
            .expect("token-status checker should initialize");
        assert_eq!(version_checker.check(&test_claims), JwtTokenStatus::Stale);

        let _: usize = connection
            .del((&denylist_key, &subject_version_key, &tenant_version_key))
            .expect("clean integration-test keys");
    }
}
