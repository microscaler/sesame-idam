//! Pooled blocking Redis connections for Sesame's request paths.
//!
//! Until 26 Oct 2026 every Redis helper on a request path (refresh-token rotation, authorization codes, OIDC
//! authorize state, OTP, abuse guard, owner-transfer OTP) opened a new connection for each call: TCP, AUTH and the
//! database selection through the data plane's Envoy before the one command it needed, about four round trips, and
//! `/session/refresh` made seven such calls (p50 200 ms in pp; Redis accepted ~11 connections a minute from an
//! almost idle Sesame). [`pooled_connection`] lends an already-open connection instead and takes it back when the
//! guard drops.
//!
//! Why sharing connections is safe here: a connection carries no per-request state. Every connection is opened on
//! the database in `REDIS_URL` with the same credentials, and Sesame's helpers never change connection state
//! (no `SELECT`, `WATCH`/`MULTI`, `CLIENT`, `SUBSCRIBE` or scripts on these connections). A borrowed connection
//! therefore behaves exactly like a fresh one. A connection that saw any error, or that Redis reports closed, is
//! never returned to the pool; one idle longer than [`IDLE_CHECK`] is checked with `PING` before it is lent.
//! Nothing is cached: every command still goes to Redis, so revocation, reuse detection and one-time codes read
//! the store as before. Connect, read and write are bounded by [`timeout`] (previously unbounded), and a Redis
//! failure still surfaces as an error to the caller, which fails closed as it did.

use std::ops::Deref;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use redis::{Cmd, ConnectionLike, RedisResult, Value};

/// Connections kept open for reuse per process.
const MAX_IDLE: usize = 16;
/// A connection idle longer than this is `PING`ed before it is lent.
pub const IDLE_CHECK: Duration = Duration::from_secs(30);
/// Default connect/read/write bound (`SESAME_REDIS_TIMEOUT_MS`).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(2);

/// The connect/read/write bound for pooled connections (`SESAME_REDIS_TIMEOUT_MS`, default 2000).
#[must_use]
pub fn timeout() -> Duration {
    static T: OnceLock<Duration> = OnceLock::new();
    *T.get_or_init(|| {
        std::env::var("SESAME_REDIS_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
            .filter(|&ms| ms > 0)
            .map_or(DEFAULT_TIMEOUT, Duration::from_millis)
    })
}

struct Idle {
    conn: redis::Connection,
    since: Instant,
}

/// A process-wide set of open connections to one Redis URL.
pub struct RedisPool {
    client: redis::Client,
    idle: Mutex<Vec<Idle>>,
    timeout: Duration,
}

impl RedisPool {
    /// A pool for `url`; nothing is connected until the first borrow.
    ///
    /// # Errors
    ///
    /// Returns an error when `url` is not a valid Redis URL.
    pub fn new(url: &str, timeout: Duration) -> RedisResult<Self> {
        Ok(Self {
            client: redis::Client::open(url)?,
            idle: Mutex::new(Vec::new()),
            timeout,
        })
    }

    fn connect(&self) -> RedisResult<redis::Connection> {
        let conn = self.client.get_connection_with_timeout(self.timeout)?;
        conn.set_read_timeout(Some(self.timeout))?;
        conn.set_write_timeout(Some(self.timeout))?;
        Ok(conn)
    }

    /// Borrow an open connection (an idle one, or a new one).
    ///
    /// # Errors
    ///
    /// Returns the connect error when no idle connection is usable and a new one cannot be opened.
    pub fn get(&self) -> RedisResult<PooledConnection<'_>> {
        loop {
            // The lock covers only the pop: no I/O while it is held.
            let candidate = self.idle.lock().ok().and_then(|mut idle| idle.pop());
            let Some(Idle { mut conn, since }) = candidate else {
                break;
            };
            if !conn.is_open() {
                continue;
            }
            if since.elapsed() >= IDLE_CHECK && !conn.check_connection() {
                continue;
            }
            return Ok(PooledConnection {
                pool: self,
                conn: Some(conn),
                broken: false,
            });
        }
        Ok(PooledConnection {
            pool: self,
            conn: Some(self.connect()?),
            broken: false,
        })
    }

    fn give_back(&self, conn: redis::Connection) {
        if let Ok(mut idle) = self.idle.lock() {
            if idle.len() < MAX_IDLE {
                idle.push(Idle {
                    conn,
                    since: Instant::now(),
                });
            }
        }
    }

    #[cfg(test)]
    fn idle_len(&self) -> usize {
        self.idle.lock().map(|idle| idle.len()).unwrap_or(0)
    }
}

/// A borrowed connection; returned to its pool on drop unless it saw an error or was closed.
pub struct PooledConnection<'a> {
    pool: &'a RedisPool,
    conn: Option<redis::Connection>,
    broken: bool,
}

impl PooledConnection<'_> {
    fn conn(&self) -> &redis::Connection {
        self.conn
            .as_ref()
            .unwrap_or_else(|| unreachable!("a pooled connection is present until drop"))
    }

    fn conn_mut(&mut self) -> &mut redis::Connection {
        self.conn
            .as_mut()
            .unwrap_or_else(|| unreachable!("a pooled connection is present until drop"))
    }

    fn track<T>(&mut self, result: RedisResult<T>) -> RedisResult<T> {
        if result.is_err() {
            self.broken = true;
        }
        result
    }
}

// No DerefMut: every command goes through the ConnectionLike impl below, which marks the connection on any error.
impl Deref for PooledConnection<'_> {
    type Target = redis::Connection;

    fn deref(&self) -> &redis::Connection {
        self.conn()
    }
}

impl ConnectionLike for PooledConnection<'_> {
    fn req_packed_command(&mut self, cmd: &[u8]) -> RedisResult<Value> {
        let result = self.conn_mut().req_packed_command(cmd);
        self.track(result)
    }

    fn req_packed_commands(
        &mut self,
        cmd: &[u8],
        offset: usize,
        count: usize,
    ) -> RedisResult<Vec<Value>> {
        let result = self.conn_mut().req_packed_commands(cmd, offset, count);
        self.track(result)
    }

    fn req_command(&mut self, cmd: &Cmd) -> RedisResult<Value> {
        let result = self.conn_mut().req_command(cmd);
        self.track(result)
    }

    fn get_db(&self) -> i64 {
        self.conn().get_db()
    }

    fn supports_pipelining(&self) -> bool {
        self.conn().supports_pipelining()
    }

    fn check_connection(&mut self) -> bool {
        let ok = self.conn_mut().check_connection();
        if !ok {
            self.broken = true;
        }
        ok
    }

    fn is_open(&self) -> bool {
        self.conn().is_open()
    }
}

impl Drop for PooledConnection<'_> {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            if !self.broken && conn.is_open() {
                self.pool.give_back(conn);
            }
        }
    }
}

/// The process-wide pool for `REDIS_URL` (default `redis://127.0.0.1:6379`, as the per-call helpers had).
///
/// # Errors
///
/// Returns an error when `REDIS_URL` is not a valid Redis URL.
pub fn shared() -> RedisResult<&'static RedisPool> {
    static POOL: OnceLock<RedisPool> = OnceLock::new();
    if let Some(pool) = POOL.get() {
        return Ok(pool);
    }
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let pool = RedisPool::new(&url, timeout())?;
    Ok(POOL.get_or_init(|| pool))
}

/// Borrow a connection from the process-wide pool for `REDIS_URL`.
///
/// # Errors
///
/// Returns an error when `REDIS_URL` is invalid or Redis cannot be reached.
pub fn pooled_connection() -> RedisResult<PooledConnection<'static>> {
    shared()?.get()
}

#[cfg(test)]
mod tests {
    use super::*;
    use redis::Commands;

    fn test_url() -> Option<String> {
        std::env::var("TEST_REDIS_URL")
            .or_else(|_| std::env::var("REDIS_URL"))
            .ok()
    }

    #[test]
    fn invalid_url_is_rejected() {
        assert!(RedisPool::new("not a url", DEFAULT_TIMEOUT).is_err());
    }

    #[test]
    fn unreachable_redis_fails_within_the_bound() {
        let pool =
            RedisPool::new("redis://127.0.0.1:1", Duration::from_millis(100)).expect("valid URL");
        let started = Instant::now();
        assert!(pool.get().is_err());
        assert!(started.elapsed() < Duration::from_secs(2));
        assert_eq!(pool.idle_len(), 0);
    }

    #[test]
    fn a_connection_is_reused_and_an_errored_one_is_not() {
        let Some(url) = test_url() else {
            eprintln!("skipping: TEST_REDIS_URL/REDIS_URL is not set");
            return;
        };
        let pool = RedisPool::new(&url, DEFAULT_TIMEOUT).expect("valid URL");
        let Ok(mut conn) = pool.get() else {
            eprintln!("skipping: Redis is not reachable");
            return;
        };
        let key = format!("redis-pool-test:{}", uuid::Uuid::new_v4());
        let _: () = conn.set_ex(&key, "1", 30).expect("SET");
        drop(conn);
        assert_eq!(pool.idle_len(), 1, "a clean connection goes back");

        let mut conn = pool.get().expect("borrow again");
        assert_eq!(pool.idle_len(), 0, "the idle connection was lent");
        let value: Option<String> = conn.get(&key).expect("GET");
        assert_eq!(value.as_deref(), Some("1"));
        // A command error (wrong type) marks the connection: it is dropped, not returned.
        let err: RedisResult<Vec<String>> = conn.smembers(&key);
        assert!(err.is_err());
        drop(conn);
        assert_eq!(pool.idle_len(), 0, "an errored connection is not returned");

        let mut conn = pool.get().expect("a fresh connection");
        let _: () = conn.del(&key).expect("DEL");
    }
}
