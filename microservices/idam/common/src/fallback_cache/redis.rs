//! Redis helper functions for the fallback cache.
//!
//! Private helpers for reading/writing cache entries in Redis.

use super::cache::FallbackCache;
use redis::Commands;

/// Read a cached decision from Redis.
///
/// Returns `Ok(None)` on cache miss or Redis error (fail-open).
pub fn redis_get(
    cache: &FallbackCache,
    key: &str,
) -> Result<Option<super::types::AuthzDecision>, Box<dyn std::error::Error + Send + Sync>> {
    let mut con = match ::redis::Client::open(cache.redis_url()) {
        Ok(client) => match client.get_connection() {
            Ok(con) => con,
            Err(e) => {
                tracing::warn!(error = %e, "Redis connection failed (fallback to authz-core)");
                return Ok(None);
            }
        },
        Err(e) => {
            tracing::warn!(error = %e, "Redis client creation failed (fallback to authz-core)");
            return Ok(None);
        }
    };

    match con.get::<_, Option<String>>(key) {
        Ok(Some(json_str)) => {
            match super::types::AuthzDecision::from_json(&json_str) {
                Ok(decision) => {
                    cache.metrics().inc_cache_hit();
                    Ok(Some(decision))
                }
                Err(e) => {
                    // Corrupted cache entry — treat as miss, will be overwritten
                    tracing::warn!(error = %e, key = ?key, "corrupted cache entry, treating as miss");
                    Ok(None)
                }
            }
        }
        Ok(None) => Ok(None), // Cache miss
        Err(e) => {
            tracing::warn!(error = %e, "Redis GET failed (fallback to authz-core)");
            Ok(None)
        }
    }
}

/// Store a decision in Redis with TTL.
pub fn redis_set(
    cache: &FallbackCache,
    key: &str,
    decision: &super::types::AuthzDecision,
    ttl_secs: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut con = match ::redis::Client::open(cache.redis_url()) {
        Ok(client) => match client.get_connection() {
            Ok(con) => con,
            Err(e) => {
                tracing::warn!(error = %e, "Redis connection failed on SET");
                return Err(e.into());
            }
        },
        Err(e) => {
            tracing::warn!(error = %e, "Redis client creation failed on SET");
            return Err(e.into());
        }
    };

    let json = decision.to_json();
    let _: () = con.set_ex(key, json, ttl_secs)?;
    Ok(())
}

/// Get the current Redis DBSIZE for `cache_size` metric.
pub fn redis_db_size(
    cache: &FallbackCache,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let mut con = match ::redis::Client::open(cache.redis_url()) {
        Ok(client) => match client.get_connection() {
            Ok(con) => con,
            Err(_) => return Ok(0),
        },
        Err(_) => return Ok(0),
    };

    match ::redis::cmd("DBSIZE").query::<i64>(&mut con) {
        Ok(size) => Ok(size as u64),
        Err(_) => Ok(0),
    }
}
