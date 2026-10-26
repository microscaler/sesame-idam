//! Refresh-token sessions in Redis: one record per live session (26 Oct 2026). Shared by identity-login-service
//! (creates a session at sign-in, ends it at logout) and identity-session-service (rotates it).
//!
//! # Why
//!
//! Until 26 Oct 2026 a sign-in wrote `refresh:{jti}` (30 days, whether or not the session was ever used again) and
//! a `family:{id}` set; every rotation deleted the old record, wrote a new one, added the new jti to the set (which
//! only grew) and wrote `denylist:{old jti}` for 24 hours: about nine sequential Redis commands, and keys proportional
//! to sign-ins over 30 days plus rotations over a day. Reuse detection was dead: it checked the denylist only after
//! finding the old record, which the rotation had already deleted, so a replayed rotated token got `invalid_grant`
//! and its session lived on.
//!
//! # The record
//!
//! `rfam:{family_id}` (a hash; the family id is the session id, carried in every refresh JWT) holds the current
//! refresh jti `cur`, the session's `sub`, `sid`, `client_id`, `tenant_id`, `scopes` and absolute expiry `abs`, and,
//! for [`Policy::grace_secs`] after a rotation only, the replaced jti `prev`, its successor tokens `succ` and
//! `prev_until` (field TTLs, Redis 7.4 `HEXPIRE`: the successor tokens leave Redis when the grace ends). The key's TTL
//! is the idle window ([`Policy::idle_secs`], `JWT_REFRESH_IDLE_SECS`, 8 hours by default: at least the BFFs' own session idle
//! window, so a BFF session never outlives its refresh token), renewed by every
//! rotation and capped at `abs`. At most two refresh tokens of a session are ever valid (the current one, and the
//! previous one for the grace window), an abandoned session disappears within the idle window, and the key count is
//! the number of sessions active within it. No sweeper.
//!
//! # Rules ([`classify`], and the compare-and-swap script for concurrent requests)
//!
//! * the current token rotates: the session moves to a new jti atomically;
//! * the token just replaced, presented again within the grace window (a lost response, a second tab, a concurrent
//!   request), gets the same successor back - never a second pair;
//! * any other token of a live session - older, or the previous one after the grace - is reuse: the session is
//!   deleted, so every token of it stops working (refresh-token reuse detection, OAuth 2.0 Security BCP 4.14);
//! * a session that is gone (logged out, revoked, idle, past `abs`) refuses every token.
//!
//! Compatibility: tokens issued before this change have a `refresh:{jti}` record. It is honoured as before and turned
//! into a session record on its first rotation (absolute expiry: the old token's own `exp`), and its old keys are
//! deleted then; otherwise old keys age out on their own TTLs. Nothing is bulk-deleted.

use std::collections::HashMap;
use std::sync::LazyLock;

use redis::{RedisResult, Script};
use serde::{Deserialize, Serialize};

/// Prefix of the per-session record: `rfam:{family_id}`.
pub const SESSION_KEY_PREFIX: &str = "rfam";
/// Pre-change per-token record prefix (`refresh:{jti}`), read for compatibility only.
pub const LEGACY_TOKEN_PREFIX: &str = "refresh";
/// Pre-change family set prefix (`family:{id}`), deleted on adoption/revocation only.
pub const LEGACY_FAMILY_PREFIX: &str = "family";
/// Pre-change rotation receipts (`idam:rotation_receipt:{jti}`, 60 s), read for compatibility only.
pub const LEGACY_RECEIPT_PREFIX: &str = "idam:rotation_receipt";
const DEFAULT_IDLE_SECS: i64 = 8 * 3600;
const DEFAULT_GRACE_SECS: i64 = 60;
/// Version-key TTL for issued access tokens (as `VersionStore::from_url`).
const VERSION_TTL_SECS: i64 = 360;

/// Session lifetimes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    /// A session not refreshed for this long ends (`JWT_REFRESH_IDLE_SECS`, default 8 h).
    pub idle_secs: i64,
    /// How long the replaced token may be presented again for the same successor (`JWT_REFRESH_GRACE_SECS`, default
    /// 60 s, at most 60 s).
    pub grace_secs: i64,
}

impl Policy {
    /// From the environment (read once per process).
    #[must_use]
    pub fn from_env() -> Self {
        static P: LazyLock<Policy> = LazyLock::new(|| {
            let num = |k: &str, d: i64| {
                std::env::var(k)
                    .ok()
                    .and_then(|v| v.trim().parse::<i64>().ok())
                    .filter(|&s| s > 0)
                    .unwrap_or(d)
            };
            Policy {
                idle_secs: num("JWT_REFRESH_IDLE_SECS", DEFAULT_IDLE_SECS),
                grace_secs: num("JWT_REFRESH_GRACE_SECS", DEFAULT_GRACE_SECS).min(60),
            }
        });
        *P
    }

    /// The session record's TTL at `now`: the idle window capped at the absolute expiry (0: already over).
    #[must_use]
    pub fn session_ttl(&self, now: i64, absolute_expiry: i64) -> i64 {
        self.idle_secs.min(absolute_expiry - now).max(0)
    }

    /// The `exp` of a refresh token issued at `now`.
    #[must_use]
    pub fn token_exp(&self, now: i64, absolute_expiry: i64) -> i64 {
        (now + self.idle_secs).min(absolute_expiry)
    }
}

/// A refresh token's session data (the JSON of the pre-change `refresh:{jti}` records has the same fields).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionToken {
    pub jti: String,
    pub sub: String,
    pub sid: String,
    pub family_id: String,
    #[serde(default)]
    pub iat: i64,
    #[serde(default)]
    pub exp: i64,
    pub client_id: String,
    #[serde(default)]
    pub tenant_id: String,
    #[serde(default)]
    pub scopes: String,
}

/// A live session as stored.
#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    pub family_id: String,
    pub cur: String,
    pub prev: Option<String>,
    pub succ: Option<serde_json::Value>,
    pub prev_until: i64,
    pub sub: String,
    pub sid: String,
    pub client_id: String,
    pub tenant_id: String,
    pub scopes: String,
    /// Absolute expiry (unix seconds).
    pub abs: i64,
}

impl Session {
    fn from_hash(family_id: &str, h: &HashMap<String, String>) -> Option<Self> {
        let get = |k: &str| h.get(k).cloned().unwrap_or_default();
        let cur = get("cur");
        if cur.is_empty() {
            return None;
        }
        Some(Self {
            family_id: family_id.to_string(),
            cur,
            prev: h.get("prev").cloned().filter(|p| !p.is_empty()),
            succ: h.get("succ").and_then(|s| serde_json::from_str(s).ok()),
            prev_until: get("prev_until").parse().unwrap_or(0),
            sub: get("sub"),
            sid: get("sid"),
            client_id: get("client_id"),
            tenant_id: get("tenant_id"),
            scopes: get("scopes"),
            abs: get("abs").parse().unwrap_or(0),
        })
    }

    /// The current token's data.
    #[must_use]
    pub fn current(&self) -> SessionToken {
        SessionToken {
            jti: self.cur.clone(),
            sub: self.sub.clone(),
            sid: self.sid.clone(),
            family_id: self.family_id.clone(),
            iat: 0,
            exp: self.abs,
            client_id: self.client_id.clone(),
            tenant_id: self.tenant_id.clone(),
            scopes: self.scopes.clone(),
        }
    }
}

/// What a presented refresh token is.
#[derive(Debug, Clone, PartialEq)]
pub enum Presented {
    /// The session's current token.
    Current(Session),
    /// A token issued before session records (its `refresh:{jti}` record is still there).
    Legacy(SessionToken),
    /// The token just replaced, within the grace window: the successor issued for it.
    Replay(serde_json::Value),
    /// Any other token of a live session: reuse.
    Superseded(Session),
    /// No live session knows this token.
    Unknown,
}

/// Decide what the presented `jti` is, from one consistent read (pure).
#[must_use]
pub fn classify(
    jti: &str,
    session: Option<Session>,
    legacy: Option<SessionToken>,
    legacy_receipt: Option<serde_json::Value>,
    now: i64,
) -> Presented {
    if let Some(l) = legacy.filter(|l| l.jti == jti) {
        return Presented::Legacy(l);
    }
    match session {
        Some(s) if s.abs <= now => Presented::Unknown,
        Some(s) if s.cur == jti => Presented::Current(s),
        Some(s) if s.prev.as_deref() == Some(jti) && s.prev_until > now && s.succ.is_some() => {
            Presented::Replay(s.succ.unwrap_or_default())
        }
        Some(s) => Presented::Superseded(s),
        // No session: only a receipt from the pre-change rotation can still answer (it lives 60 s).
        None => legacy_receipt.map_or(Presented::Unknown, Presented::Replay),
    }
}

fn session_key(family_id: &str) -> String {
    format!("{SESSION_KEY_PREFIX}:{family_id}")
}
fn legacy_key(jti: &str) -> String {
    format!("{LEGACY_TOKEN_PREFIX}:{jti}")
}
fn legacy_family_key(family_id: &str) -> String {
    format!("{LEGACY_FAMILY_PREFIX}:{family_id}")
}
fn legacy_receipt_key(jti: &str) -> String {
    format!("{LEGACY_RECEIPT_PREFIX}:{jti}")
}

/// Round trip 1 of a rotation: the session, any pre-change record and receipt for `jti`, and the subject's token
/// version (initialised and kept for the access token's lifetime, exactly as `VersionStore::issue_version`).
///
/// # Errors
///
/// Returns the Redis error; callers fail closed.
pub fn inspect(family_id: &str, jti: &str, sub: &str, now: i64) -> RedisResult<(Presented, u64)> {
    let mut conn = crate::redis_pool::pooled_connection()?;
    let version_key = crate::token_versioning::subject_key(sub);
    let (hash, legacy, receipt, _, version, _): (
        HashMap<String, String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<u64>,
        i64,
    ) = redis::pipe()
        .cmd("HGETALL")
        .arg(session_key(family_id))
        .cmd("GET")
        .arg(legacy_key(jti))
        .cmd("GET")
        .arg(legacy_receipt_key(jti))
        .cmd("SET")
        .arg(&version_key)
        .arg(1_u64)
        .arg("NX")
        .cmd("GET")
        .arg(&version_key)
        .cmd("EXPIRE")
        .arg(&version_key)
        .arg(VERSION_TTL_SECS)
        .query(&mut conn)?;
    Ok((
        classify(
            jti,
            Session::from_hash(family_id, &hash),
            legacy.and_then(|j| serde_json::from_str(&j).ok()),
            receipt.and_then(|r| serde_json::from_str(&r).ok()),
            now,
        ),
        version.unwrap_or(1),
    ))
}

/// Result of the compare-and-swap.
#[derive(Debug, Clone, PartialEq)]
pub enum Advance {
    /// The session now points at the new token.
    Rotated,
    /// A concurrent request rotated this token first: its successor.
    Replay(serde_json::Value),
    /// The token had been superseded: reuse; the session is deleted.
    Reuse,
    /// The session (or pre-change record) is gone.
    Gone,
}

/// Compare-and-swap. KEYS: session, legacy record of old jti, legacy family set, legacy receipt of old jti.
/// ARGV: mode (cur|legacy), old jti, new jti, now, key ttl, successor json, grace, then for legacy the session fields.
static ADVANCE: LazyLock<Script> = LazyLock::new(|| {
    Script::new(
        r"
local fam, legacy, legacy_set, legacy_receipt = KEYS[1], KEYS[2], KEYS[3], KEYS[4]
local mode, old, new, now, ttl, succ, grace = ARGV[1], ARGV[2], ARGV[3], tonumber(ARGV[4]), ARGV[5], ARGV[6], tonumber(ARGV[7])
local function replay_of(jti)
  local f = redis.call('HMGET', fam, 'prev', 'succ', 'prev_until')
  if f[1] == jti and f[2] and tonumber(f[3] or '0') > now then return f[2] end
  return nil
end
local cur = redis.call('HGET', fam, 'cur')
if mode == 'cur' then
  if cur ~= old then
    local r = replay_of(old)
    if r then return {'replay', r} end
    if cur then
      redis.call('DEL', fam)
      return {'reuse'}
    end
    return {'gone'}
  end
else
  if redis.call('EXISTS', legacy) == 0 then
    local r = replay_of(old) or redis.call('GET', legacy_receipt)
    if r then return {'replay', r} end
    return {'gone'}
  end
  redis.call('DEL', fam)
  redis.call('HSET', fam, 'sub', ARGV[8], 'sid', ARGV[9], 'client_id', ARGV[10], 'tenant_id', ARGV[11],
             'scopes', ARGV[12], 'abs', ARGV[13], 'created', now)
end
redis.call('HSET', fam, 'cur', new, 'prev', old, 'succ', succ, 'prev_until', now + grace, 'rot_at', now)
redis.call('HINCRBY', fam, 'gen', 1)
redis.call('EXPIRE', fam, ttl)
redis.call('HEXPIRE', fam, grace, 'FIELDS', 3, 'prev', 'succ', 'prev_until')
redis.call('DEL', legacy, legacy_set)
return {'ok'}
",
    )
});

/// Advance the session `family_id` (or adopt the pre-change token `legacy`) from `old_jti` to `new_jti`, keeping
/// `successor` for `old_jti` for the grace window. Atomic: of concurrent requests with the same token, one rotates and
/// the others get its successor.
///
/// # Errors
///
/// Returns the Redis error; callers fail closed.
#[allow(clippy::too_many_arguments)]
pub fn advance(
    family_id: &str,
    old_jti: &str,
    new_jti: &str,
    legacy: Option<&SessionToken>,
    absolute_expiry: i64,
    now: i64,
    successor: &serde_json::Value,
    policy: Policy,
) -> RedisResult<Advance> {
    let mut conn = crate::redis_pool::pooled_connection()?;
    let mut inv = ADVANCE.prepare_invoke();
    inv.key(session_key(family_id))
        .key(legacy_key(old_jti))
        .key(legacy_family_key(family_id))
        .key(legacy_receipt_key(old_jti))
        .arg(if legacy.is_some() { "legacy" } else { "cur" })
        .arg(old_jti)
        .arg(new_jti)
        .arg(now)
        .arg(policy.session_ttl(now, absolute_expiry).max(1))
        .arg(successor.to_string())
        .arg(policy.grace_secs.max(1));
    if let Some(l) = legacy {
        inv.arg(&l.sub)
            .arg(&l.sid)
            .arg(&l.client_id)
            .arg(&l.tenant_id)
            .arg(&l.scopes)
            .arg(absolute_expiry);
    }
    let reply: Vec<String> = inv.invoke(&mut conn)?;
    Ok(match reply.first().map(String::as_str) {
        Some("ok") => Advance::Rotated,
        Some("replay") => reply
            .get(1)
            .and_then(|r| serde_json::from_str(r).ok())
            .map_or(Advance::Gone, Advance::Replay),
        Some("reuse") => Advance::Reuse,
        _ => Advance::Gone,
    })
}

/// KEYS: session, legacy record, legacy family set. ARGV: legacy token prefix.
static REVOKE: LazyLock<Script> = LazyLock::new(|| {
    Script::new(
        r"
local n = redis.call('DEL', KEYS[1], KEYS[2])
for _, j in ipairs(redis.call('SMEMBERS', KEYS[3])) do
  n = n + redis.call('DEL', ARGV[1] .. ':' .. j)
end
redis.call('DEL', KEYS[3])
return n
",
    )
});

/// End the session `family_id` (presented token `jti`): its record, and for pre-change tokens the presented token's
/// record and those listed in its old family set. Every token of the session is refused afterwards.
///
/// # Errors
///
/// Returns the Redis error.
pub fn revoke(family_id: &str, jti: &str) -> RedisResult<()> {
    let mut conn = crate::redis_pool::pooled_connection()?;
    let _: i64 = REVOKE
        .key(session_key(family_id))
        .key(legacy_key(jti))
        .key(legacy_family_key(family_id))
        .arg(LEGACY_TOKEN_PREFIX)
        .invoke(&mut conn)?;
    Ok(())
}

/// Start a session at sign-in, its first refresh token current (one atomic round trip).
///
/// # Errors
///
/// Returns the Redis error.
pub fn create(
    token: &SessionToken,
    absolute_expiry: i64,
    now: i64,
    policy: Policy,
) -> RedisResult<()> {
    let mut conn = crate::redis_pool::pooled_connection()?;
    let key = session_key(&token.family_id);
    let _: () = redis::pipe()
        .atomic()
        .cmd("DEL")
        .arg(&key)
        .ignore()
        .cmd("HSET")
        .arg(&key)
        .arg(&[
            ("cur", token.jti.as_str()),
            ("sub", token.sub.as_str()),
            ("sid", token.sid.as_str()),
            ("client_id", token.client_id.as_str()),
            ("tenant_id", token.tenant_id.as_str()),
            ("scopes", token.scopes.as_str()),
        ])
        .ignore()
        .cmd("HSET")
        .arg(&key)
        .arg("abs")
        .arg(absolute_expiry)
        .arg("created")
        .arg(now)
        .arg("gen")
        .arg(1)
        .ignore()
        .cmd("EXPIRE")
        .arg(&key)
        .arg(policy.session_ttl(now, absolute_expiry).max(1))
        .ignore()
        .query(&mut conn)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use redis::Commands;

    const NOW: i64 = 1_800_000_000;
    const POLICY: Policy = Policy {
        idle_secs: 8 * 3600,
        grace_secs: 60,
    };

    fn session(cur: &str, prev: Option<&str>, prev_until: i64) -> Session {
        Session {
            family_id: "f1".into(),
            cur: cur.into(),
            prev: prev.map(str::to_string),
            succ: prev.map(|_| serde_json::json!({"refresh_token": "succ"})),
            prev_until,
            sub: "u1".into(),
            sid: "f1".into(),
            client_id: "c1".into(),
            tenant_id: "t1".into(),
            scopes: "openid".into(),
            abs: NOW + 1000,
        }
    }

    fn token(jti: &str, family: &str) -> SessionToken {
        SessionToken {
            jti: jti.into(),
            sub: "u1".into(),
            sid: family.into(),
            family_id: family.into(),
            iat: NOW,
            exp: NOW + 10,
            client_id: "c1".into(),
            tenant_id: "t1".into(),
            scopes: "openid".into(),
        }
    }

    #[test]
    fn classify_follows_the_rules() {
        assert!(matches!(
            classify(
                "j2",
                Some(session("j2", Some("j1"), NOW + 30)),
                None,
                None,
                NOW
            ),
            Presented::Current(_)
        ));
        assert!(matches!(
            classify(
                "j1",
                Some(session("j2", Some("j1"), NOW + 30)),
                None,
                None,
                NOW
            ),
            Presented::Replay(_)
        ));
        // The previous token after the grace, and any older token: reuse.
        assert!(matches!(
            classify("j1", Some(session("j2", Some("j1"), NOW)), None, None, NOW),
            Presented::Superseded(_)
        ));
        assert!(matches!(
            classify(
                "j0",
                Some(session("j2", Some("j1"), NOW + 30)),
                None,
                None,
                NOW
            ),
            Presented::Superseded(_)
        ));
        // Past the absolute expiry, or gone: refused.
        let mut s = session("j2", None, 0);
        s.abs = NOW;
        assert_eq!(classify("j2", Some(s), None, None, NOW), Presented::Unknown);
        assert_eq!(classify("j2", None, None, None, NOW), Presented::Unknown);
        // A pre-change record answers for its own jti only.
        assert!(matches!(
            classify("old", None, Some(token("old", "f1")), None, NOW),
            Presented::Legacy(_)
        ));
        assert_eq!(
            classify("x", None, Some(token("old", "f1")), None, NOW),
            Presented::Unknown
        );
    }

    #[test]
    fn the_session_ttl_is_the_idle_window_capped_at_the_absolute_expiry() {
        assert_eq!(POLICY.session_ttl(NOW, NOW + 30 * 86_400), 8 * 3600);
        assert_eq!(POLICY.session_ttl(NOW, NOW + 100), 100);
        assert_eq!(POLICY.session_ttl(NOW, NOW - 1), 0);
        assert_eq!(POLICY.token_exp(NOW, NOW + 100), NOW + 100);
        assert!(Policy::from_env().grace_secs <= 60);
    }

    // ---- against a real Redis 7.4+ (REDIS_URL); skipped when none is reachable

    fn redis_ready() -> bool {
        std::env::var("REDIS_URL").is_ok() && crate::redis_pool::pooled_connection().is_ok()
    }
    fn now() -> i64 {
        chrono::Utc::now().timestamp()
    }
    fn fresh(cur: &str, abs_in: i64, policy: Policy) -> String {
        let fid = format!("test-{}", uuid::Uuid::new_v4());
        create(&token(cur, &fid), now() + abs_in, now(), policy).expect("create");
        fid
    }
    fn look(fid: &str, jti: &str) -> Presented {
        inspect(fid, jti, "u1", now()).expect("inspect").0
    }
    fn succ(name: &str) -> serde_json::Value {
        serde_json::json!({ "refresh_token": name })
    }

    #[test]
    fn concurrent_double_use_yields_one_rotation_and_the_same_successor() {
        if !redis_ready() {
            eprintln!("skipping: no Redis");
            return;
        }
        for _ in 0..25 {
            let fid = fresh("j1", 3600, POLICY);
            let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
            let handles: Vec<_> = (0..2)
                .map(|i| {
                    let (fid, b) = (fid.clone(), barrier.clone());
                    std::thread::spawn(move || {
                        assert!(matches!(look(&fid, "j1"), Presented::Current(_)));
                        b.wait();
                        let t = now();
                        advance(
                            &fid,
                            "j1",
                            &format!("j2-{i}"),
                            None,
                            t + 3600,
                            t,
                            &succ(&format!("j2-{i}")),
                            POLICY,
                        )
                        .expect("advance")
                    })
                })
                .collect();
            let results: Vec<Advance> = handles.into_iter().map(|h| h.join().unwrap()).collect();
            assert_eq!(
                results.iter().filter(|r| **r == Advance::Rotated).count(),
                1,
                "{results:?}"
            );
            let Some(Advance::Replay(winner)) = results
                .iter()
                .find(|r| matches!(r, Advance::Replay(_)))
                .cloned()
            else {
                panic!("the loser must get the winner's successor: {results:?}");
            };
            let winner_jti = winner["refresh_token"].as_str().unwrap().to_string();
            assert!(matches!(look(&fid, &winner_jti), Presented::Current(_)));
            assert_eq!(look(&fid, "j1"), Presented::Replay(winner));
            revoke(&fid, "j1").unwrap();
        }
    }

    #[test]
    fn replay_within_grace_returns_the_successor_and_after_grace_revokes_the_session() {
        if !redis_ready() {
            eprintln!("skipping: no Redis");
            return;
        }
        let short = Policy {
            idle_secs: 3600,
            grace_secs: 1,
        };
        let fid = fresh("j1", 3600, short);
        let t = now();
        assert_eq!(
            advance(&fid, "j1", "j2", None, t + 3600, t, &succ("j2"), short).unwrap(),
            Advance::Rotated
        );
        assert_eq!(look(&fid, "j1"), Presented::Replay(succ("j2")));
        std::thread::sleep(std::time::Duration::from_millis(2100));
        // The grace fields expired with the grace (field TTLs): the successor tokens are no longer in Redis.
        let mut conn = crate::redis_pool::pooled_connection().unwrap();
        let fields: Vec<Option<String>> = redis::cmd("HMGET")
            .arg(session_key(&fid))
            .arg("prev")
            .arg("succ")
            .query(&mut conn)
            .unwrap();
        assert_eq!(fields, vec![None, None]);
        drop(conn);
        assert!(matches!(look(&fid, "j1"), Presented::Superseded(_)));
        let t = now();
        assert_eq!(
            advance(&fid, "j1", "jx", None, t + 3600, t, &succ("jx"), short).unwrap(),
            Advance::Reuse
        );
        // The whole session is gone: the current token is refused too.
        assert_eq!(look(&fid, "j2"), Presented::Unknown);
    }

    #[test]
    fn an_older_token_is_reuse_and_ends_the_session() {
        if !redis_ready() {
            eprintln!("skipping: no Redis");
            return;
        }
        let fid = fresh("j1", 3600, POLICY);
        let t = now();
        assert_eq!(
            advance(&fid, "j1", "j2", None, t + 3600, t, &succ("j2"), POLICY).unwrap(),
            Advance::Rotated
        );
        assert_eq!(
            advance(&fid, "j2", "j3", None, t + 3600, t, &succ("j3"), POLICY).unwrap(),
            Advance::Rotated
        );
        assert!(matches!(look(&fid, "j1"), Presented::Superseded(_)));
        assert_eq!(
            advance(&fid, "j1", "jx", None, t + 3600, t, &succ("jx"), POLICY).unwrap(),
            Advance::Reuse
        );
        assert_eq!(look(&fid, "j3"), Presented::Unknown);
    }

    #[test]
    fn an_idle_session_expires_and_the_absolute_cap_holds() {
        if !redis_ready() {
            eprintln!("skipping: no Redis");
            return;
        }
        let idle = Policy {
            idle_secs: 1,
            grace_secs: 1,
        };
        let fid = fresh("j1", 3600, idle);
        std::thread::sleep(std::time::Duration::from_millis(2100));
        assert_eq!(look(&fid, "j1"), Presented::Unknown, "idle window passed");

        // Absolute cap: a long idle window, 2 s of absolute lifetime; rotation cannot extend it.
        let fid = fresh("j1", 2, POLICY);
        let t = now();
        assert_eq!(
            advance(&fid, "j1", "j2", None, t + 2, t, &succ("j2"), POLICY).unwrap(),
            Advance::Rotated
        );
        let mut conn = crate::redis_pool::pooled_connection().unwrap();
        let ttl: i64 = conn.ttl(session_key(&fid)).unwrap();
        assert!(ttl <= 2, "capped at the absolute expiry: {ttl}");
        drop(conn);
        std::thread::sleep(std::time::Duration::from_millis(3100));
        assert_eq!(look(&fid, "j2"), Presented::Unknown);
    }

    #[test]
    fn a_pre_change_token_rotates_once_and_its_old_keys_go() {
        if !redis_ready() {
            eprintln!("skipping: no Redis");
            return;
        }
        let fid = format!("test-{}", uuid::Uuid::new_v4());
        let old = token("old1", &fid);
        let mut conn = crate::redis_pool::pooled_connection().unwrap();
        let _: () = conn
            .set_ex(
                legacy_key("old1"),
                serde_json::to_string(&old).unwrap(),
                600,
            )
            .unwrap();
        let _: () = conn.sadd(legacy_family_key(&fid), "old1").unwrap();
        drop(conn);
        assert!(matches!(look(&fid, "old1"), Presented::Legacy(_)));
        let t = now();
        assert_eq!(
            advance(
                &fid,
                "old1",
                "n1",
                Some(&old),
                t + 600,
                t,
                &succ("n1"),
                POLICY
            )
            .unwrap(),
            Advance::Rotated
        );
        let mut conn = crate::redis_pool::pooled_connection().unwrap();
        let gone: (bool, bool) = (
            conn.exists(legacy_key("old1")).unwrap(),
            conn.exists(legacy_family_key(&fid)).unwrap(),
        );
        assert_eq!(gone, (false, false));
        drop(conn);
        assert!(
            matches!(look(&fid, "n1"), Presented::Current(ref s) if s.client_id == "c1" && s.tenant_id == "t1")
        );
        assert_eq!(look(&fid, "old1"), Presented::Replay(succ("n1")));
        revoke(&fid, "n1").unwrap();
        assert_eq!(look(&fid, "n1"), Presented::Unknown);
    }
}
