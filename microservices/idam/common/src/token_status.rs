//! Redis-backed dynamic JWT status checks for BRRTRouter.
//!
//! Signature and standard claims are validated by `JwksBearerProvider`. This module supplies
//! the Sesame-specific read side: targeted `jti` denylisting plus subject/tenant version checks.
//! Redis errors fail closed. A small bounded cache collapses BRRTRouter's validate-then-extract
//! sequence without creating a meaningful revocation window.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use brrtrouter::security::{JwtTokenStatus, JwtTokenStatusChecker};
use dashmap::DashMap;
use redis::Client;
use serde_json::Value;

use crate::token_versioning::{subject_key, tenant_key};

const DEFAULT_CACHE_TTL: Duration = Duration::from_millis(100);
const DEFAULT_MAX_CACHE_ENTRIES: usize = 10_000;

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
    connection: Mutex<Option<redis::Connection>>,
}

impl RedisTokenStatusLookup {
    fn new(redis_url: &str) -> Result<Self, String> {
        let client = Client::open(redis_url)
            .map_err(|error| format!("invalid REDIS_URL for token status: {error}"))?;
        Ok(Self {
            client,
            connection: Mutex::new(None),
        })
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
            .connection
            .lock()
            .map_err(|_| "token-status Redis connection lock poisoned".to_string())?;
        if guard.is_none() {
            *guard = Some(
                self.client
                    .get_connection()
                    .map_err(|error| format!("token-status Redis unavailable: {error}"))?,
            );
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

#[derive(Debug, Clone, Copy)]
struct CachedStatus {
    status: JwtTokenStatus,
    checked_at: Instant,
}

/// Sesame denylist and token-version checker used by every JWKS consumer.
pub struct SesameTokenStatusChecker {
    lookup: Arc<dyn TokenStatusLookup>,
    cache: DashMap<String, CachedStatus>,
    cache_ttl: Duration,
    max_cache_entries: usize,
}

impl SesameTokenStatusChecker {
    /// Construct from an explicit Redis URL.
    ///
    /// # Errors
    ///
    /// Returns an error when the Redis URL is invalid.
    pub fn from_redis_url(redis_url: &str) -> Result<Self, String> {
        Ok(Self {
            lookup: Arc::new(RedisTokenStatusLookup::new(redis_url)?),
            cache: DashMap::new(),
            cache_ttl: DEFAULT_CACHE_TTL,
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
    fn with_lookup(lookup: Arc<dyn TokenStatusLookup>, cache_ttl: Duration) -> Self {
        Self {
            lookup,
            cache: DashMap::new(),
            cache_ttl,
            max_cache_entries: DEFAULT_MAX_CACHE_ENTRIES,
        }
    }

    fn cached(&self, jti: &str) -> Option<JwtTokenStatus> {
        let entry = self.cache.get(jti)?;
        if entry.checked_at.elapsed() < self.cache_ttl {
            return Some(entry.status);
        }
        drop(entry);
        self.cache.remove(jti);
        None
    }

    fn cache_status(&self, jti: &str, status: JwtTokenStatus) {
        if matches!(
            status,
            JwtTokenStatus::Unavailable | JwtTokenStatus::Invalid
        ) {
            return;
        }
        if self.cache.len() >= self.max_cache_entries {
            if let Some(oldest_key) = self.cache.iter().next().map(|entry| entry.key().clone()) {
                self.cache.remove(&oldest_key);
            }
        }
        self.cache.insert(
            jti.to_string(),
            CachedStatus {
                status,
                checked_at: Instant::now(),
            },
        );
    }
}

impl JwtTokenStatusChecker for SesameTokenStatusChecker {
    fn check(&self, claims: &Value) -> JwtTokenStatus {
        let Some(jti) = claims
            .get("jti")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
        else {
            return JwtTokenStatus::Invalid;
        };
        let Some(subject) = claims
            .get("sub")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
        else {
            return JwtTokenStatus::Invalid;
        };
        let Some(tenant_id) = claims
            .get("tenant_id")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
        else {
            return JwtTokenStatus::Invalid;
        };
        let Some(claimed_version) = claims.get("ver").and_then(Value::as_u64) else {
            return JwtTokenStatus::Invalid;
        };

        if let Some(status) = self.cached(jti) {
            return status;
        }

        let authoritative = match self.lookup.lookup(jti, subject, tenant_id) {
            Ok(status) => status,
            Err(error) => {
                tracing::error!(event = "token_status_unavailable", error = %error);
                return JwtTokenStatus::Unavailable;
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
        self.cache_status(jti, status);
        status
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
            Arc::clone(&lookup) as Arc<dyn TokenStatusLookup>,
            Duration::from_secs(1),
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
    fn active_result_is_cached_for_validate_then_extract() {
        let (checker, lookup) = checker(Ok(AuthoritativeStatus {
            revoked: false,
            subject_version: 1,
            tenant_version: 1,
        }));
        assert_eq!(checker.check(&claims(1)), JwtTokenStatus::Active);
        assert_eq!(checker.check(&claims(1)), JwtTokenStatus::Active);
        assert_eq!(lookup.calls.load(Ordering::SeqCst), 1);
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
        let mut connection = client
            .get_connection()
            .expect("configured test Redis must be available");
        let checker = SesameTokenStatusChecker::from_redis_url(&redis_url)
            .expect("token-status checker should initialize");

        let _: usize = connection
            .del((&denylist_key, &subject_version_key, &tenant_version_key))
            .expect("clear integration-test keys");
        assert_eq!(checker.check(&test_claims), JwtTokenStatus::Active);

        let _: () = connection
            .set_ex(&denylist_key, "1", 60)
            .expect("write denylist entry");
        std::thread::sleep(DEFAULT_CACHE_TTL + Duration::from_millis(20));
        assert_eq!(checker.check(&test_claims), JwtTokenStatus::Revoked);

        let _: usize = connection.del(&denylist_key).expect("clear denylist entry");
        let _: () = connection
            .set_ex(&subject_version_key, 2_u64, 60)
            .expect("write subject version");
        std::thread::sleep(DEFAULT_CACHE_TTL + Duration::from_millis(20));
        assert_eq!(checker.check(&test_claims), JwtTokenStatus::Stale);

        let _: usize = connection
            .del((&denylist_key, &subject_version_key, &tenant_version_key))
            .expect("clean integration-test keys");
    }
}
