// BRRTRouter: user-owned
//! `POST /mcp/token` — an agent exchanges a signed client assertion for a short-lived access token.
//!
//! Agents come from the GitOps registry ([`sesame_common::agents`]); none is creatable through the API. The
//! assertion is an ES256 JWT signed with the agent's registered key (`iss` = `sub` = agent id, `aud` = this issuer,
//! one-time `jti`). The token issued carries only the agent's own audiences, roles and scope, `user_type = "agent"`,
//! and lives a few minutes; there is no refresh token — the agent signs a new assertion instead.
//!
//! Kill switches: disabling the agent in the registry stops new tokens (the ConfigMap reaches the pod within a minute
//! or two); live tokens end within the agent's ttl (default 5 minutes). Bumping the agent's subject version revokes
//! live tokens only at consumers that run Sesame's token-status check.
//!
//! Hardening: the agent id must be a UUID and the assertion at most [`MAX_CREDENTIAL_BYTES`]; audit records the
//! canonical id, never the caller's spelling.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_common::agents::{self, Agent, AgentError};
use sesame_common::audit::{AuditEventType, AuditLogEntry};
use sesame_common::jwt::{AccessClaimsBuilder, SesameAuthzClaimsBuilder};
use sesame_common::token_versioning::VersionStore;
use sesame_idam_identity_session_service_gen::handlers::mcp_token::Request;
use uuid::Uuid;

use crate::audit::EMITTER;
use crate::services::token_issuer::SIGNER;

/// Largest client assertion accepted. A real one is about 400 bytes.
pub const MAX_CREDENTIAL_BYTES: usize = 4096;

#[handler(McpTokenController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    // Malformed input is refused before any lookup, crypto, Redis or audit write.
    let (Ok(agent_uuid), true) = (
        Uuid::parse_str(&req.data.agent_id),
        req.data.credential.len() <= MAX_CREDENTIAL_BYTES && req.data.x_tenant_id.len() <= 64,
    ) else {
        return HttpJson::new(
            401,
            error_body("invalid_client", "Agent authentication failed"),
        );
    };
    let agent_id = agent_uuid.hyphenated().to_string();
    let tenant = req.data.x_tenant_id.clone();
    match issue(&agent_id, &tenant, &req.data.credential) {
        Ok((agent, body)) => {
            audit(&tenant, &agent.id, "allowed", &agent.name, None);
            tracing::info!(agent = %agent.name, sponsor = %agent.sponsor, "agent token issued");
            HttpJson::ok(body)
        }
        Err(Refusal::Agent(err)) => {
            audit(
                &tenant,
                &agent_id,
                "denied",
                err.reason(),
                Some(&err.to_string()),
            );
            tracing::warn!(agent_id = %agent_id, reason = err.reason(), detail = %err, "agent token refused");
            if matches!(err, AgentError::Registry(_)) {
                return HttpJson::new(
                    503,
                    error_body("temporarily_unavailable", "Agent registry unavailable"),
                );
            }
            HttpJson::new(
                401,
                error_body("invalid_client", "Agent authentication failed"),
            )
        }
        Err(Refusal::Replayed) => {
            audit(&tenant, &agent_id, "denied", "assertion_replayed", None);
            tracing::warn!(agent_id = %agent_id, "agent assertion replayed");
            HttpJson::new(
                401,
                error_body("invalid_client", "Agent authentication failed"),
            )
        }
        Err(Refusal::Internal(e)) => {
            tracing::error!(agent_id = %agent_id, error = %e, "agent token issuance failed");
            HttpJson::new(
                503,
                error_body("temporarily_unavailable", "Could not issue the token"),
            )
        }
    }
}

enum Refusal {
    Agent(AgentError),
    Replayed,
    Internal(String),
}

impl From<AgentError> for Refusal {
    fn from(e: AgentError) -> Self {
        Self::Agent(e)
    }
}

fn issue(
    agent_id: &str,
    tenant: &str,
    assertion: &str,
) -> Result<(Agent, serde_json::Value), Refusal> {
    let registry = agents::registry()?;
    let agent = registry
        .find(agent_id)
        .ok_or(AgentError::UnknownAgent)?
        .clone();
    if !agent.enabled {
        return Err(AgentError::Disabled.into());
    }
    if agent.tenant != tenant {
        return Err(AgentError::TenantMismatch.into());
    }
    let issuer = sesame_common::deployment::issuer();
    let now = chrono::Utc::now().timestamp();
    let verified = agents::verify_assertion(&agent, assertion, &issuer, now)?;

    // One use per assertion: kept until it could no longer be accepted anyway. Fails closed without Redis.
    let ttl =
        u64::try_from((verified.exp - now).max(0) + agents::ASSERTION_LEEWAY_SECS).unwrap_or(60);
    let key = format!(
        "idam:agent_assertion:{}:{}",
        agent.id.to_ascii_lowercase(),
        verified.jti
    );
    match crate::redis::claim_once(&key, ttl) {
        Ok(true) => {}
        Ok(false) => return Err(Refusal::Replayed),
        Err(e) => return Err(Refusal::Internal(format!("replay store: {e}"))),
    }

    // The version is the kill switch for live tokens: bumping the agent's subject version revokes them.
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let version = match VersionStore::from_url(&redis_url) {
        Ok(store) => match store.issue_version(&agent.id) {
            Ok((v, _)) => v,
            Err(e) => return Err(Refusal::Internal(format!("version store: {e}"))),
        },
        Err(e) => return Err(Refusal::Internal(format!("version store: {e}"))),
    };

    let ttl_secs = agent.ttl_secs();
    let sx = SesameAuthzClaimsBuilder::new()
        .tenant(&agent.tenant)
        .portal("agent")
        .roles(agent.roles.clone())
        .permissions(vec![])
        .build()
        .map_err(|e| Refusal::Internal(e.to_string()))?;
    let jti = Uuid::new_v4().to_string();
    let claims = AccessClaimsBuilder::new()
        .iss(issuer)
        .sub(&agent.id)
        .aud(agent.audiences.clone())
        .client_id(format!("agent:{}", agent.name))
        .scope(agent.scope.clone())
        .exp(now + ttl_secs)
        .nbf(now)
        .iat(now)
        .jti(jti.clone())
        .ver(version)
        .sid(format!("agent-{jti}"))
        .tenant_id(&agent.tenant)
        .user_id(&agent.id)
        .user_type("agent")
        .sx(sx)
        .build()
        .map_err(|e| Refusal::Internal(e.to_string()))?;
    let access_token = SIGNER
        .sign_access_claims(&claims)
        .map_err(|e| Refusal::Internal(e.to_string()))?;

    let body = serde_json::json!({
        "access_token": access_token,
        "token_type": "Bearer",
        "expires_in": ttl_secs,
        "scope": agent.scope,
    });
    Ok((agent, body))
}

fn audit(tenant: &str, agent_id: &str, result: &str, reason: &str, detail: Option<&str>) {
    let event = if result == "allowed" {
        AuditEventType::JwtIssued
    } else {
        AuditEventType::ValidationFailed
    };
    let mut entry = AuditLogEntry::new(event, "identity-session-service")
        .tenant_id(tenant.to_string())
        .user_id(agent_id.to_string())
        .decision_source("agent_token")
        .result(result)
        .reason(reason);
    if let Some(d) = detail {
        entry = entry.error(d.to_string());
    }
    if let Ok(entry) = entry.build() {
        EMITTER.emit(entry);
    }
}

fn error_body(error: &str, description: &str) -> serde_json::Value {
    serde_json::json!({ "error": error, "error_description": description })
}
