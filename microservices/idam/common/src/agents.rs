//! Agent principals: non-human callers (an operations assistant, a CI job) that sign in with a key, never a password.
//!
//! Agents are declared in a registry file delivered by GitOps ([`AGENTS_FILE_ENV`], a mounted `ConfigMap`): each entry
//! names the agent, its tenant, the person who sponsors it, its P-256 public key and exactly what its tokens may reach
//! (`audiences`, `roles`, `scope`). Adding, narrowing or disabling an agent is a commit; nothing here is writable
//! through an API.
//!
//! An agent proves possession of its key with a short client assertion (RFC 7523 style): an ES256 JWT with
//! `iss` = `sub` = the agent id, `aud` = the Sesame issuer, a unique `jti` and a lifetime of at most
//! [`MAX_ASSERTION_LIFETIME_SECS`]. [`verify_assertion`] checks all of that; the caller enforces one-time use of the
//! `jti` and issues the access token.
//!
//! ```yaml
//! agents:
//!   - id: 6f1c2b8e-0000-4000-8000-000000000001
//!     name: claude-ops
//!     tenant: pricewhisperer
//!     sponsor: charles@pricewhisperer.ai
//!     enabled: true
//!     audiences: [pricewhisperer-platform]
//!     roles: [platform-observer]
//!     scope: platform:read
//!     ttl_secs: 300
//!     public_jwk: { kty: EC, crv: P-256, x: "...", y: "...", kid: claude-ops-1 }
//! ```

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use serde::Deserialize;

/// Environment variable naming the agent registry file.
pub const AGENTS_FILE_ENV: &str = "SESAME_AGENTS_FILE";

/// Longest client assertion accepted (seconds between `iat` and `exp`).
pub const MAX_ASSERTION_LIFETIME_SECS: i64 = 300;

/// Clock skew tolerated on assertion timestamps.
pub const ASSERTION_LEEWAY_SECS: i64 = 30;

/// Access-token lifetime when an agent sets none.
pub const DEFAULT_AGENT_TTL_SECS: i64 = 300;

/// Longest access token an agent can be given, whatever its entry says.
pub const MAX_AGENT_TTL_SECS: i64 = 900;

/// A P-256 public key in JWK form.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct EcPublicJwk {
    pub kty: String,
    pub crv: String,
    pub x: String,
    pub y: String,
    #[serde(default)]
    pub kid: Option<String>,
}

/// One registered agent.
#[derive(Debug, Clone, Deserialize)]
pub struct Agent {
    /// Agent id (UUID): the `sub` of its tokens and the `agent_id` it presents.
    pub id: String,
    /// Short name, used in `client_id` (`agent:<name>`) and logs.
    pub name: String,
    /// The only tenant the agent may act in.
    pub tenant: String,
    /// The person accountable for the agent (recorded in audit, never in tokens).
    pub sponsor: String,
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    /// Audiences minted into its tokens. Keep these narrow: they decide which services accept the token.
    pub audiences: Vec<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    pub scope: String,
    #[serde(default)]
    pub ttl_secs: Option<i64>,
    pub public_jwk: EcPublicJwk,
}

fn enabled_default() -> bool {
    true
}

impl Agent {
    /// Access-token lifetime: the entry's, else [`DEFAULT_AGENT_TTL_SECS`], never above [`MAX_AGENT_TTL_SECS`].
    #[must_use]
    pub fn ttl_secs(&self) -> i64 {
        self.ttl_secs
            .unwrap_or(DEFAULT_AGENT_TTL_SECS)
            .clamp(1, MAX_AGENT_TTL_SECS)
    }
}

/// The registry file's contents.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AgentRegistry {
    #[serde(default)]
    pub agents: Vec<Agent>,
}

/// Why an agent was refused. [`AgentError::reason`] is the fixed token used in audit and metrics.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AgentError {
    #[error("agent registry unavailable: {0}")]
    Registry(String),
    #[error("unknown agent")]
    UnknownAgent,
    #[error("agent disabled")]
    Disabled,
    #[error("agent belongs to another tenant")]
    TenantMismatch,
    #[error("invalid client assertion: {0}")]
    InvalidAssertion(&'static str),
}

impl AgentError {
    /// Low-cardinality reason token.
    #[must_use]
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Registry(_) => "registry_unavailable",
            Self::UnknownAgent => "unknown_agent",
            Self::Disabled => "agent_disabled",
            Self::TenantMismatch => "tenant_mismatch",
            Self::InvalidAssertion(_) => "invalid_assertion",
        }
    }
}

impl AgentRegistry {
    /// Parse and check a registry document: unique ids, P-256 keys, at least one audience per agent.
    ///
    /// # Errors
    ///
    /// [`AgentError::Registry`] when the document is malformed or an entry is unusable.
    pub fn parse(yaml: &str) -> Result<Self, AgentError> {
        let reg: Self =
            serde_yaml::from_str(yaml).map_err(|e| AgentError::Registry(e.to_string()))?;
        let mut seen = std::collections::HashSet::new();
        for a in &reg.agents {
            if uuid::Uuid::parse_str(&a.id).is_err() {
                return Err(AgentError::Registry(format!(
                    "agent {}: id is not a UUID",
                    a.name
                )));
            }
            if !seen.insert(a.id.to_ascii_lowercase()) {
                return Err(AgentError::Registry(format!(
                    "agent id {} listed twice",
                    a.id
                )));
            }
            if a.public_jwk.kty != "EC" || a.public_jwk.crv != "P-256" {
                return Err(AgentError::Registry(format!(
                    "agent {}: key must be EC P-256",
                    a.name
                )));
            }
            if a.audiences.is_empty() || a.audiences.iter().any(|s| s.trim().is_empty()) {
                return Err(AgentError::Registry(format!(
                    "agent {}: needs at least one audience",
                    a.name
                )));
            }
        }
        Ok(reg)
    }

    /// The agent with this id (case-insensitive UUID match).
    #[must_use]
    pub fn find(&self, id: &str) -> Option<&Agent> {
        self.agents.iter().find(|a| a.id.eq_ignore_ascii_case(id))
    }
}

/// The registry from [`AGENTS_FILE_ENV`], re-read when the file changes (a `ConfigMap` update needs no restart).
/// No file configured means no agents.
///
/// # Errors
///
/// [`AgentError::Registry`] when the configured file cannot be read or parsed.
pub fn registry() -> Result<Arc<AgentRegistry>, AgentError> {
    let Some(path) = std::env::var_os(AGENTS_FILE_ENV).map(PathBuf::from) else {
        return Ok(Arc::new(AgentRegistry::default()));
    };
    CACHE.load(&path)
}

static CACHE: RegistryCache = RegistryCache {
    inner: Mutex::new(None),
};

struct RegistryCache {
    inner: Mutex<Option<(PathBuf, Option<SystemTime>, Arc<AgentRegistry>)>>,
}

impl RegistryCache {
    fn load(&self, path: &Path) -> Result<Arc<AgentRegistry>, AgentError> {
        let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
        let mut slot = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((p, m, reg)) = slot.as_ref() {
            if p == path && modified.is_some() && *m == modified {
                return Ok(Arc::clone(reg));
            }
        }
        let text = std::fs::read_to_string(path)
            .map_err(|e| AgentError::Registry(format!("{}: {e}", path.display())))?;
        let reg = Arc::new(AgentRegistry::parse(&text)?);
        *slot = Some((path.to_path_buf(), modified, Arc::clone(&reg)));
        Ok(reg)
    }
}

/// A client assertion that passed every check but the one-time-use of its `jti`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedAssertion {
    pub jti: String,
    pub exp: i64,
}

#[derive(Deserialize)]
struct AssertionClaims {
    iss: String,
    sub: String,
    exp: i64,
    iat: i64,
    #[serde(default)]
    jti: String,
}

/// Check an agent's client assertion: ES256 signature by the agent's registered key, `iss` = `sub` = agent id,
/// `aud` = `expected_aud`, not expired, issued no later than now (plus leeway), lifetime at most
/// [`MAX_ASSERTION_LIFETIME_SECS`], and a `jti` present.
///
/// # Errors
///
/// [`AgentError::InvalidAssertion`] naming the failed check.
pub fn verify_assertion(
    agent: &Agent,
    assertion: &str,
    expected_aud: &str,
    now: i64,
) -> Result<VerifiedAssertion, AgentError> {
    let key = DecodingKey::from_ec_components(&agent.public_jwk.x, &agent.public_jwk.y)
        .map_err(|_| AgentError::InvalidAssertion("registered key unusable"))?;
    let mut validation = Validation::new(Algorithm::ES256);
    validation.set_audience(&[expected_aud]);
    validation.set_required_spec_claims(&["exp", "iat", "aud", "iss", "sub"]);
    // Expiry is checked below against the caller's clock, so tests and callers agree on "now".
    validation.validate_exp = false;
    validation.validate_nbf = false;
    let data =
        jsonwebtoken::decode::<AssertionClaims>(assertion, &key, &validation).map_err(|e| {
            use jsonwebtoken::errors::ErrorKind;
            AgentError::InvalidAssertion(match e.kind() {
                ErrorKind::InvalidSignature => "bad signature",
                ErrorKind::InvalidAlgorithm => "algorithm must be ES256",
                ErrorKind::InvalidAudience => "wrong audience",
                ErrorKind::MissingRequiredClaim(_) => "missing claim",
                _ => "malformed",
            })
        })?;
    let c = data.claims;
    if !c.iss.eq_ignore_ascii_case(&agent.id) || !c.sub.eq_ignore_ascii_case(&agent.id) {
        return Err(AgentError::InvalidAssertion(
            "iss and sub must be the agent id",
        ));
    }
    if c.exp <= now - ASSERTION_LEEWAY_SECS {
        return Err(AgentError::InvalidAssertion("expired"));
    }
    if c.iat > now + ASSERTION_LEEWAY_SECS {
        return Err(AgentError::InvalidAssertion("issued in the future"));
    }
    if c.exp - c.iat > MAX_ASSERTION_LIFETIME_SECS || c.exp < c.iat {
        return Err(AgentError::InvalidAssertion("lifetime too long"));
    }
    if c.jti.is_empty() || c.jti.len() > 128 {
        return Err(AgentError::InvalidAssertion("jti missing"));
    }
    Ok(VerifiedAssertion {
        jti: c.jti,
        exp: c.exp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use p256::ecdsa::SigningKey;
    use p256::pkcs8::EncodePrivateKey;

    const ID: &str = "6f1c2b8e-0000-4000-8000-000000000001";
    const AUD: &str = "https://sesame.example";

    fn keypair() -> (EncodingKey, EcPublicJwk) {
        let sk = SigningKey::random(&mut rand::rngs::OsRng);
        let point = sk.verifying_key().to_encoded_point(false);
        let jwk = EcPublicJwk {
            kty: "EC".into(),
            crv: "P-256".into(),
            x: URL_SAFE_NO_PAD.encode(point.x().unwrap()),
            y: URL_SAFE_NO_PAD.encode(point.y().unwrap()),
            kid: None,
        };
        let der = sk.to_pkcs8_der().unwrap();
        (EncodingKey::from_ec_der(der.as_bytes()), jwk)
    }

    fn agent(jwk: EcPublicJwk) -> Agent {
        Agent {
            id: ID.into(),
            name: "claude-ops".into(),
            tenant: "pricewhisperer".into(),
            sponsor: "owner@example.com".into(),
            enabled: true,
            audiences: vec!["pricewhisperer-platform".into()],
            roles: vec!["platform-observer".into()],
            scope: "platform:read".into(),
            ttl_secs: None,
            public_jwk: jwk,
        }
    }

    fn sign(key: &EncodingKey, claims: &serde_json::Value) -> String {
        encode(&Header::new(Algorithm::ES256), claims, key).unwrap()
    }

    fn claims(now: i64) -> serde_json::Value {
        serde_json::json!({"iss": ID, "sub": ID, "aud": AUD, "iat": now, "exp": now + 60, "jti": "j1"})
    }

    #[test]
    fn accepts_a_well_formed_assertion() {
        let (key, jwk) = keypair();
        let now = 1_800_000_000;
        let ok = verify_assertion(&agent(jwk), &sign(&key, &claims(now)), AUD, now).unwrap();
        assert_eq!(
            ok,
            VerifiedAssertion {
                jti: "j1".into(),
                exp: now + 60
            }
        );
    }

    #[test]
    fn refuses_another_key_audience_subject_or_stale_assertion() {
        let (key, jwk) = keypair();
        let (other_key, _) = keypair();
        let a = agent(jwk);
        let now = 1_800_000_000;
        let reason = |t: &str| verify_assertion(&a, t, AUD, now).unwrap_err();

        assert_eq!(
            reason(&sign(&other_key, &claims(now))),
            AgentError::InvalidAssertion("bad signature")
        );
        let mut c = claims(now);
        c["aud"] = "https://elsewhere".into();
        assert_eq!(
            reason(&sign(&key, &c)),
            AgentError::InvalidAssertion("wrong audience")
        );
        let mut c = claims(now);
        c["sub"] = "someone-else".into();
        assert!(matches!(
            reason(&sign(&key, &c)),
            AgentError::InvalidAssertion(_)
        ));
        let mut c = claims(now - 600);
        c["exp"] = (now - 540).into();
        assert_eq!(
            reason(&sign(&key, &c)),
            AgentError::InvalidAssertion("expired")
        );
        let mut c = claims(now);
        c["exp"] = (now + 3600).into();
        assert_eq!(
            reason(&sign(&key, &c)),
            AgentError::InvalidAssertion("lifetime too long")
        );
        let mut c = claims(now);
        c["jti"] = "".into();
        assert_eq!(
            reason(&sign(&key, &c)),
            AgentError::InvalidAssertion("jti missing")
        );
    }

    #[test]
    fn refuses_a_symmetric_or_unsigned_token() {
        let (_, jwk) = keypair();
        let now = 1_800_000_000;
        let hs = encode(
            &Header::new(Algorithm::HS256),
            &claims(now),
            &EncodingKey::from_secret(b"guess"),
        )
        .unwrap();
        assert!(verify_assertion(&agent(jwk), &hs, AUD, now).is_err());
        let unsigned = format!(
            "{}.{}.",
            URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#),
            URL_SAFE_NO_PAD.encode(claims(now).to_string())
        );
        assert!(verify_assertion(&agent(keypair().1), &unsigned, AUD, now).is_err());
    }

    #[test]
    fn registry_checks_entries_and_clamps_ttl() {
        let doc = |id: &str, crv: &str, aud: &str| {
            format!(
                "agents:\n  - id: {id}\n    name: a\n    tenant: t\n    sponsor: s\n    audiences: [{aud}]\n    scope: r\n    ttl_secs: 99999\n    public_jwk: {{kty: EC, crv: {crv}, x: AA, y: AA}}\n"
            )
        };
        let reg = AgentRegistry::parse(&doc(ID, "P-256", "x")).unwrap();
        let a = reg.find(&ID.to_uppercase()).unwrap();
        assert!(a.enabled);
        assert_eq!(a.ttl_secs(), MAX_AGENT_TTL_SECS);
        assert!(AgentRegistry::parse(&doc("not-a-uuid", "P-256", "x")).is_err());
        assert!(AgentRegistry::parse(&doc(ID, "P-384", "x")).is_err());
        assert!(AgentRegistry::parse(&doc(ID, "P-256", "\"\"")).is_err());
        assert!(AgentRegistry::parse(&format!(
            "{}{}",
            doc(ID, "P-256", "x"),
            &doc(ID, "P-256", "x")[8..]
        ))
        .is_err());
    }
}
