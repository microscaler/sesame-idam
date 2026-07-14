//! CSRF `state` storage for OAuth login initiation.

use anyhow::{Context, Result};
use redis::Commands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const STATE_KEY_PREFIX: &str = "oauth_state:";
const STATE_TTL_SECS: u64 = 600;

/// Payload bound to a single OAuth `state` parameter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OAuthState {
    pub tenant_id: String,
    pub provider: String,
    pub redirect_uri: String,
}

fn redis_conn() -> Result<redis::Connection> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let client = redis::Client::open(redis_url.as_str())?;
    Ok(client.get_connection()?)
}

/// Persist OAuth state and return the opaque `state` token for the authorize URL.
///
/// # Errors
///
/// Returns an error when Redis is unavailable.
pub fn store_oauth_state(payload: &OAuthState) -> Result<String> {
    let state = Uuid::new_v4().to_string();
    let key = format!("{STATE_KEY_PREFIX}{state}");
    let json = serde_json::to_string(payload).context("serialize oauth state")?;
    let mut conn = redis_conn()?;
    let _: () = conn
        .set_ex(&key, json, STATE_TTL_SECS)
        .context("redis set_ex oauth state")?;
    Ok(state)
}

/// Load and delete OAuth state (single use).
///
/// # Errors
///
/// Returns an error when Redis is unavailable or the state is missing/expired.
pub fn consume_oauth_state(state: &str) -> Result<OAuthState> {
    if state.trim().is_empty() {
        anyhow::bail!("missing state");
    }
    let key = format!("{STATE_KEY_PREFIX}{state}");
    let mut conn = redis_conn()?;
    let json: Option<String> = conn.get(&key).context("redis get oauth state")?;
    let Some(json) = json else {
        anyhow::bail!("invalid or expired state");
    };
    let _: () = conn.del(&key).context("redis del oauth state")?;
    let payload: OAuthState = serde_json::from_str(&json).context("deserialize oauth state")?;
    Ok(payload)
}
