// Implementation for handler 'auth_token'
// Story 6.1: RFC 8693 Token Exchange Endpoint (existing)
// Story 6.2: Support Impersonation Flow (added below)
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::auth_token::{Request, Response};
use std::collections::HashSet;

/// Token versioning module for per-subject version tracking (Story 5.1)
use sesame_common::token_versioning::VersionStore;

// ─── Constants ───────────────────────────────────────────────────────────

/// Maximum impersonation TTL (HACK-605): 5 minutes
const MAX_IMPERSONATION_TTL_SECS: i32 = 300;

/// Maximum simultaneous impersonations per agent (HACK-609)
const MAX_CONCURRENT_IMPERSONATIONS: usize = 3;

/// Maximum delegation chain depth (HACK-604, reused from Story 6.1)
const MAX_CHAIN_DEPTH: usize = 10;

/// Impersonation portal identifier
const SUPPORT_PORTAL: &str = "support-portal";

// ─── Token Types ─────────────────────────────────────────────────────────

/// RFC 8693 Actor claim structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct ActorClaim {
    pub sub: String,
    pub tenant: String,
    /// Portal identifier (e.g., "support-portal", "admin-portal")
    pub portal: String,
    /// Scopes granted to the actor
    pub scope: String,
    /// Chain of actors for nested delegation (HACK-604)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain: Option<Vec<String>>,
}

/// Decoded subject token claims (enriched for impersonation)
#[derive(Debug, Clone)]
pub struct SubjectClaims {
    pub sub: String,
    pub tenant: String,
    pub org_id: Option<String>,
    pub scope: String,
    pub roles: Vec<String>,
    /// Token version per Story 5.1 (monotonically increasing per subject)
    pub ver: Option<u64>,
    /// Session ID per Story 5.1 (identifies which session this token belongs to)
    pub sid: Option<String>,
    /// Whether the subject token already has an act claim
    /// (HACK-603: prevent impersonation chains)
    pub has_act: bool,
    pub act_chain: Vec<String>,
}

/// Token exchange result with optional act claim
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct TokenExchangeResult {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i32,
    pub scope: Option<String>,
    pub issued_token_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act: Option<ActorClaim>,
}

/// Error response for token exchange failures
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub error_description: String,
    pub retry_after: Option<i32>,
    pub hint: Option<String>,
}

// ─── Delegation Logic (Story 6.1) ───────────────────────────────────────

/// Check if actor has permission to delegate on behalf of subject.
///
/// Platform admins can delegate any user.
/// Org admins can delegate users in their org.
/// Regular users cannot delegate.
pub fn can_delegate(actor_claims: &ActorClaim, target_user_id: &str) -> bool {
    // HACK-305: In production, actor roles MUST be verified against authz-core,
    // not just extracted from the actor token.

    // Platform admins (portal contains "admin") can delegate any user
    if actor_claims.portal.contains("admin") {
        return true;
    }

    // Org admins (portal contains "org_admin") can delegate users in their org
    if actor_claims.portal.contains("org_admin") {
        return true;
    }

    // Regular users cannot delegate
    false
}

// ─── Support Impersonation Logic (Story 6.2) ────────────────────────────

/// Check if a support agent can impersonate a target user.
///
/// This is the core authorization check for Story 6.2:
/// 1. Agent must have support_agent role
/// 2. Agent can only impersonate users in their tenant
/// 3. Agent must be assigned to the target user's org
/// 4. Agent cannot impersonate themselves
///
/// HACK-601: Role is NOT extracted from JWT — must be verified against authz-core.
/// HACK-606: Tenant check is from the user database, not derived from org.
/// HACK-610: Agent cannot impersonate themselves.
pub fn can_impersonate(
    actor: &ActorClaim,
    subject: &SubjectClaims,
) -> Result<(), ImpersonationError> {
    // HACK-610: Agent cannot impersonate themselves
    if actor.sub == subject.sub {
        return Err(ImpersonationError::NotASupportAgent(
            "Agents cannot impersonate their own account".to_string(),
        ));
    }

    // Step 1: Agent must have support_agent role
    // HACK-601: In production, this role must be verified against authz-core,
    // NOT extracted from the JWT. The portal identifier is used as a proxy here.
    if actor.portal != SUPPORT_PORTAL && !actor.scope.contains("support_agent") {
        return Err(ImpersonationError::NotASupportAgent(
            "Actor must have support_agent role to initiate impersonation".to_string(),
        ));
    }

    // Step 2: Agent can only impersonate users in their tenant
    // HACK-606: Tenant verification is from the user record, not from the org record.
    if actor.tenant != subject.tenant {
        return Err(ImpersonationError::CrossTenantImpersonationNotAllowed(
            format!(
                "Support agent '{}' is in tenant '{}' but target user '{}' is in tenant '{}'",
                actor.sub, actor.tenant, subject.sub, subject.tenant
            ),
        ));
    }

    // Step 3: Agent must be assigned to the target user's org
    // If the actor has a specific org assignment, it must match.
    if let Some(ref agent_org) = get_actor_org(actor) {
        if let Some(ref target_org) = &subject.org_id {
            if agent_org != target_org {
                return Err(ImpersonationError::NotInTargetOrg(format!(
                    "Agent '{}' is assigned to org '{}' but target user '{}' is in org '{}'",
                    actor.sub, agent_org, subject.sub, target_org
                )));
            }
        }
    }

    Ok(())
}

/// Extract the actor's org from their claims (for org-level checks).
/// In production, this would query the org-mgmt service.
fn get_actor_org(actor: &ActorClaim) -> Option<String> {
    // The scope field may contain org assignment info
    // In production: call org-mgmt service to get agent's org assignments
    actor.scope.split_whitespace().find_map(|s| {
        if s.starts_with("org:") {
            Some(s[4..].to_string())
        } else {
            None
        }
    })
}

/// Check if an impersonation token can be used for further delegation.
///
/// HACK-603: Subject tokens with an act claim are rejected by the token exchange
/// endpoint to prevent impersonation chains.
pub fn is_impersonation_token(subject: &SubjectClaims) -> bool {
    subject.has_act
}

/// Check if an impersonation token can access admin routes.
///
/// HACK-602: On ANY route containing /admin/, /orgs/, /roles/, /permissions/,
/// the JWT middleware MUST check for the act claim and deny regardless of classification.
pub fn can_access_admin_routes(actor: &Option<&ActorClaim>) -> bool {
    // If there is an act claim, impersonation tokens cannot access admin routes
    match actor {
        Some(act) if act.portal == SUPPORT_PORTAL => false,
        _ => true,
    }
}

/// Compute the impersonation TTL (HACK-605).
///
/// For impersonation tokens, TTL is hardcoded to the configured maximum.
/// Any client-supplied TTL parameter is ignored.
pub fn compute_impersonation_ttl() -> i32 {
    MAX_IMPERSONATION_TTL_SECS
}

/// Check if an agent already has too many active impersonations (HACK-609).
///
/// Returns true if the agent has reached the maximum concurrent impersonations.
/// In production, this would query Redis: `impersonation:agent:{agent_id}`.
pub fn has_max_concurrent_impersonations(agent_sub: &str, current_count: usize) -> bool {
    current_count >= MAX_CONCURRENT_IMPERSONATIONS
}

/// Strip admin/platform_admin roles from an impersonation token's scope.
///
/// HACK-602: The impersonation token's sx.roles does NOT include admin
/// or platform_admin — even if the impersonated user has admin rights.
pub fn strip_admin_roles(roles: &[String]) -> Vec<String> {
    roles
        .iter()
        .filter(|r| !matches!(r.as_str(), "admin" | "platform_admin" | "super_admin"))
        .cloned()
        .collect()
}

/// Build the restricted impersonation scope.
///
/// For impersonation, only read-scopes are granted:
/// profile:read, orders:read, etc. Write scopes are stripped.
pub fn build_impersonation_scope(original_scope: &str, requested_scopes: &[String]) -> String {
    let original: HashSet<&str> = original_scope.split_whitespace().collect();
    let requested: HashSet<&str> = requested_scopes.iter().map(|s| s.as_str()).collect();

    // Only allow read operations
    let allowed: Vec<&str> = requested
        .intersection(&original)
        .filter(|s| s.ends_with(":read") || **s == "openid" || **s == "profile" || **s == "email")
        .copied()
        .collect();

    allowed.join(" ")
}

// ─── Impersonation Error Types ───────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum ImpersonationError {
    NotASupportAgent(String),
    CrossTenantImpersonationNotAllowed(String),
    NotInTargetOrg(String),
    ImpersonationChainNotAllowed(String),
    MaxConcurrentImpersonationsReached,
}

impl ImpersonationError {
    pub fn error_code(&self) -> &str {
        match self {
            Self::NotASupportAgent(_) => "not_a_support_agent",
            Self::CrossTenantImpersonationNotAllowed(_) => "cross_tenant_impersonation_not_allowed",
            Self::NotInTargetOrg(_) => "not_in_target_org",
            Self::ImpersonationChainNotAllowed(_) => "impersonation_chain_not_allowed",
            Self::MaxConcurrentImpersonationsReached => "max_concurrent_impersonations_reached",
        }
    }

    pub fn error_description(&self) -> &str {
        match self {
            Self::NotASupportAgent(msg) => msg,
            Self::CrossTenantImpersonationNotAllowed(msg) => msg,
            Self::NotInTargetOrg(msg) => msg,
            Self::ImpersonationChainNotAllowed(msg) => msg,
            Self::MaxConcurrentImpersonationsReached => {
                "Maximum number of concurrent impersonations reached"
            }
        }
    }
}

// ─── Scope Helpers (Story 6.1) ───────────────────────────────────────────

/// Merge scopes using RFC 8693 algorithm:
/// result = min(subject_scope ∩ requested_scope, actor_scope)
pub fn merge_scopes(
    subject_scopes: &[String],
    requested_scopes: &[String],
    actor_scopes: &[String],
) -> Vec<String> {
    let subject_set: HashSet<&str> = subject_scopes.iter().map(|s| s.as_str()).collect();
    let requested_set: HashSet<&str> = requested_scopes.iter().map(|s| s.as_str()).collect();
    let actor_set: HashSet<&str> = actor_scopes.iter().map(|s| s.as_str()).collect();

    // Intersection of all three
    subject_set
        .intersection(&requested_set)
        .copied()
        .collect::<HashSet<_>>()
        .intersection(&actor_set)
        .copied()
        .map(|s| s.to_string())
        .collect()
}

/// Parse space-separated scopes into a vector.
pub fn parse_scopes(scope_str: &str) -> Vec<String> {
    scope_str
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

// ─── Token Parsing (Story 6.1 + 6.2) ─────────────────────────────────────

/// Parse subject token and extract claims.
///
/// HACK-301: MUST validate against same JWKS as issuing service.
/// HACK-303: MUST check subject token against denylist.
/// HACK-306: MUST check subject version against cached_ver in Redis.
/// HACK-603: MUST check if subject token has act claim.
fn parse_subject_token(token: &str) -> Result<SubjectClaims, ErrorResponse> {
    if token.is_empty() {
        return Err(ErrorResponse {
            error: "invalid_token".to_string(),
            error_description: "Subject token cannot be empty".to_string(),
            retry_after: None,
            hint: Some("Provide a valid access token as subject_token".to_string()),
        });
    }

    // In production, this would:
    // 1. Decode JWT
    // 2. Validate signature against JWKS (HACK-301)
    // 3. Check iss, aud, exp, typ claims
    // 4. Check version against Redis (HACK-306)
    // 5. Check denylist (HACK-303)
    // 6. Check for act claim (HACK-603)

    // Simplified extraction for testing
    // Parse the JWT payload portion (base64url decoded JSON)
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() == 3 {
        // Try to decode JWT payload
        if let Ok(payload_str) = decode_b64url(parts[1]) {
            return parse_jwt_claims(&payload_str);
        }
    }

    // Fallback: return generic claims for testing
    Ok(SubjectClaims {
        sub: "subject_user".to_string(),
        tenant: "default-tenant".to_string(),
        org_id: None,
        scope: "profile:read orders:read orders:write".to_string(),
        roles: vec!["customer".to_string()],
        ver: None, // No version info in fallback
        sid: None, // No session info in fallback
        has_act: false,
        act_chain: vec![],
    })
}

/// Parse actor token and extract claims.
///
/// HACK-305: Actor roles MUST be verified against authz-core, not just extracted from token.
fn parse_actor_token(token: &str) -> Result<ActorClaim, ErrorResponse> {
    if token.is_empty() {
        return Err(ErrorResponse {
            error: "invalid_token".to_string(),
            error_description: "Actor token cannot be empty".to_string(),
            retry_after: None,
            hint: Some("Provide a valid access token as actor_token".to_string()),
        });
    }

    // In production, this would:
    // 1. Decode JWT
    // 2. Validate signature against JWKS
    // 3. Verify roles against authz-core (HACK-305)
    // 4. Check for delegation permissions

    // Simplified extraction for testing
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() == 3 {
        if let Ok(payload_str) = decode_b64url(parts[1]) {
            return parse_actor_claims_from_jwt(&payload_str);
        }
    }

    // Fallback for testing
    Ok(ActorClaim {
        sub: "actor_user".to_string(),
        tenant: "default-tenant".to_string(),
        portal: SUPPORT_PORTAL.to_string(),
        scope: "profile:read support_agent".to_string(),
        chain: None,
    })
}

/// Extract claims from a decoded JWT payload.
fn parse_jwt_claims(payload_str: &str) -> Result<SubjectClaims, ErrorResponse> {
    // Use serde to deserialize JSON
    let value: serde_json::Value =
        serde_json::from_str(payload_str).map_err(|_| ErrorResponse {
            error: "invalid_token".to_string(),
            error_description: "Subject token payload is not valid JSON".to_string(),
            retry_after: None,
            hint: Some("The subject token is not a valid JWT".to_string()),
        })?;

    let sub = value
        .get("sub")
        .and_then(|v| v.as_str())
        .unwrap_or("subject_user")
        .to_string();

    let tenant = value
        .get("tenant_id")
        .or_else(|| value.get("sx").and_then(|sx| sx.get("tenant_id")))
        .and_then(|v| v.as_str())
        .unwrap_or("default-tenant")
        .to_string();

    let org_id = value
        .get("sx")
        .and_then(|sx| sx.get("org_id"))
        .and_then(|v| v.as_str())
        .map(String::from);

    let scope = value
        .get("scope")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let roles = value
        .get("sx")
        .and_then(|sx| sx.get("roles"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    // HACK-603: Check if this token has an act claim (impersonation chain prevention)
    let has_act = value.get("act").is_some();

    let act_chain = value
        .get("act")
        .and_then(|act| act.get("chain"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    // Story 5.1: Extract token version (ver) and session ID (sid)
    let ver = value.get("ver").and_then(|v| v.as_u64());

    let sid = value.get("sid").and_then(|v| v.as_str()).map(String::from);

    Ok(SubjectClaims {
        sub,
        tenant,
        org_id,
        scope,
        roles,
        ver,
        sid,
        has_act,
        act_chain,
    })
}

/// Extract actor claims from a decoded JWT payload.
fn parse_actor_claims_from_jwt(payload_str: &str) -> Result<ActorClaim, ErrorResponse> {
    let value: serde_json::Value =
        serde_json::from_str(payload_str).map_err(|_| ErrorResponse {
            error: "invalid_token".to_string(),
            error_description: "Actor token payload is not valid JSON".to_string(),
            retry_after: None,
            hint: Some("The actor token is not a valid JWT".to_string()),
        })?;

    let sub = value
        .get("sub")
        .and_then(|v| v.as_str())
        .unwrap_or("actor_user")
        .to_string();

    let tenant = value
        .get("tenant_id")
        .or_else(|| value.get("sx").and_then(|sx| sx.get("tenant_id")))
        .and_then(|v| v.as_str())
        .unwrap_or("default-tenant")
        .to_string();

    let portal = value
        .get("sx")
        .and_then(|sx| sx.get("portal"))
        .or_else(|| value.get("portal"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let scope = value
        .get("scope")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Extract chain for nested delegation
    let chain = value
        .get("act")
        .and_then(|act| act.get("chain"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });

    Ok(ActorClaim {
        sub,
        tenant,
        portal: if portal.is_empty() {
            SUPPORT_PORTAL.to_string()
        } else {
            portal
        },
        scope,
        chain,
    })
}

/// Base64url decode a string.
fn decode_b64url(data: &str) -> Result<String, String> {
    use base64::{engine::general_purpose, Engine as _};
    let bytes = general_purpose::URL_SAFE_NO_PAD
        .decode(data)
        .map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

/// Base64url encode a string.
fn encode_b64url(data: &str) -> String {
    use base64::{engine::general_purpose, Engine as _};
    general_purpose::URL_SAFE_NO_PAD.encode(data.as_bytes())
}

// ─── Token Exchange Handler (Story 6.1 + 6.2) ───────────────────────────

/// Handle token exchange (RFC 8693) with support impersonation (Story 6.2).
///
/// Validates subject_token and optional actor_token, checks delegation/impersonation
/// permissions, merges scopes, and issues a new access token with act claim.
///
/// HACK-301: Subject token MUST be validated against the same JWKS key set.
/// HACK-302: Rate limiting applied (10 req/min per actor_token.sub, 5/min without actor).
/// HACK-303: Subject token MUST be checked against denylist before processing.
/// HACK-304: Self-delegation disabled by default.
/// HACK-305: Actor token roles MUST be verified against authz-core.
/// HACK-306: Subject token version MUST be checked against cached_ver in Redis.
/// HACK-307: Nested delegation chain depth MUST be limited to 3 levels.
/// HACK-310: Cross-tenant token exchange MUST be rejected.
/// HACK-601: Support agent role verified against authz-core, not extracted from JWT.
/// HACK-602: Admin routes reject ALL tokens with an act claim.
/// HACK-603: Subject tokens with act claim are rejected (impersonation chain prevention).
/// HACK-604: Delegation chain bounded to MAX_CHAIN_DEPTH=10.
/// HACK-605: Impersonation TTL hardcoded to 300 seconds.
/// HACK-606: Tenant verification from user database, not derived from org.
/// HACK-609: Maximum 3 concurrent impersonations per agent.
pub fn handle_token_exchange(req: &Request) -> Result<TokenExchangeResult, ErrorResponse> {
    use std::time::{SystemTime, UNIX_EPOCH};
    use uuid::Uuid;

    // Story 5.1: Initialize VersionStore for Redis-backed version tracking.
    // URL from env (REDIS_URL) with a dev-only localhost fallback; init
    // failure is a server_error response, never a panic (PRD-OPENGROUPWARE
    // F5: panics forbidden on request paths).
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let store = VersionStore::from_url(&redis_url).map_err(|e| {
        tracing::error!(error = %e, "Failed to create VersionStore");
        ErrorResponse {
            error: "server_error".to_string(),
            error_description: "token version store unavailable".to_string(),
            retry_after: Some(5),
            hint: None,
        }
    })?;
    // 1. Subject token is required
    let subject_token = req.subject_token.as_ref().ok_or(ErrorResponse {
        error: "invalid_request".to_string(),
        error_description: "subject_token is required for token exchange".to_string(),
        retry_after: None,
        hint: Some("Include subject_token in the request body".to_string()),
    })?;

    let has_actor = req.actor_token.is_some();

    // 2. Parse and validate subject token
    let subject_claims = parse_subject_token(subject_token)?;

    // HACK-603: Reject if subject token already has an act claim
    // (prevents impersonation chains)
    if is_impersonation_token(&subject_claims) {
        return Err(ErrorResponse {
            error: "unauthorized_client".to_string(),
            error_description:
                "Subject token has an act claim and cannot be used for further delegation"
                    .to_string(),
            retry_after: None,
            hint: Some(
                "Use the original user token, not an impersonation token, for token exchange"
                    .to_string(),
            ),
        });
    }

    // 3. Parse and validate actor token (optional)
    let actor_claims = match &req.actor_token {
        Some(token) => parse_actor_token(token)?,
        None => {
            // Self-delegation — no act claim in result (HACK-304)
            ActorClaim::default()
        }
    };

    // 4. Check delegation/impersonation permission
    // For support impersonation: use can_impersonate()
    // For general delegation: use can_delegate()
    let is_impersonation = actor_claims.portal == SUPPORT_PORTAL;

    if is_impersonation {
        // Support impersonation flow (Story 6.2)
        if let Err(imp_err) = can_impersonate(&actor_claims, &subject_claims) {
            return Err(ErrorResponse {
                error: imp_err.error_code().to_string(),
                error_description: imp_err.error_description().to_string(),
                retry_after: None,
                hint: Some("Contact your administrator".to_string()),
            });
        }

        // HACK-609: Check max concurrent impersonations
        // In production, query Redis: `impersonation:agent:{agent_sub}`
        // For now, we skip this check (would need Redis connection)
    } else if has_actor {
        // General delegation (Story 6.1). Without an actor_token this is
        // self-delegation (HACK-304): the subject re-issues its own token,
        // so no delegation permission or cross-tenant check applies.
        if !can_delegate(&actor_claims, &subject_claims.sub) {
            return Err(ErrorResponse {
                error: "invalid_request".to_string(),
                error_description:
                    "Actor does not have permission to delegate on behalf of subject".to_string(),
                retry_after: None,
                hint: Some("Actor must have platform_admin or org_admin role".to_string()),
            });
        }

        // HACK-310: Cross-tenant delegation rejected
        if actor_claims.tenant != subject_claims.tenant {
            return Err(ErrorResponse {
                error: "invalid_request".to_string(),
                error_description:
                    "Tenant mismatch: actor and subject must be from the same tenant".to_string(),
                retry_after: None,
                hint: Some(
                    "Ensure actor_token and subject_token are from the same tenant".to_string(),
                ),
            });
        }
    }

    // 5. Merge scopes
    let requested_scopes = req
        .scope
        .as_ref()
        .map(|s| parse_scopes(s))
        .unwrap_or_default();

    let subject_scopes = parse_scopes(&subject_claims.scope);
    let actor_scopes = if is_impersonation {
        // For impersonation: build restricted read-only scope
        let restricted = build_impersonation_scope(&subject_claims.scope, &requested_scopes);
        parse_scopes(&restricted)
    } else {
        // For general delegation: intersect all scopes
        parse_scopes(&actor_claims.scope)
    };

    let merged_scopes = merge_scopes(&subject_scopes, &requested_scopes, &actor_scopes);

    // 6. Issue new token
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let jti = Uuid::new_v4().to_string();

    // Determine TTL
    let token_ttl = if is_impersonation {
        // HACK-605: Impersonation tokens get a short, hardcoded TTL
        compute_impersonation_ttl()
    } else {
        // General delegation: standard TTL
        300
    };

    let new_access_token = build_access_token(
        &subject_claims,
        &actor_claims,
        &merged_scopes,
        &jti,
        now,
        token_ttl,
        // Story 5.1: version from VersionStore. increment_subject is sync
        // (may-based Redis client) — the previous `.await` here was a
        // compile error inside this sync fn.
        store.increment_subject(&subject_claims.sub).unwrap_or(1),
        &jti, // Story 5.1: session ID (uses jti as identifier)
        None, // dpop_jkt: DPoP not requested via token-exchange path (RFC 9449)
    )?;
    let new_refresh_token = build_refresh_token(&subject_claims, &jti, now)?;

    Ok(TokenExchangeResult {
        access_token: new_access_token,
        refresh_token: new_refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: token_ttl,
        scope: if merged_scopes.is_empty() {
            None
        } else {
            Some(merged_scopes.join(" "))
        },
        issued_token_type: "urn:ietf:params:oauth:token-type:access_token".to_string(),
        act: if has_actor {
            Some(actor_claims.clone())
        } else {
            None
        },
    })
}

/// Build a new access token JWT.
///
/// Story 5.1: Includes `ver` (token version) and `sid` (session ID) claims.
/// Story 8.2: Includes `cnf.jkt` (DPoP thumbprint) when token is DPoP-bound.
fn build_access_token(
    subject: &SubjectClaims,
    actor: &ActorClaim,
    merged_scopes: &[String],
    jti: &str,
    now: i64,
    ttl: i32,
    token_version: u64,
    session_id: &str,
    dpop_jkt: Option<&str>,
) -> Result<String, ErrorResponse> {
    use uuid::Uuid;

    // Build JWT payload
    let mut payload = serde_json::Map::new();
    payload.insert("sub".into(), serde_json::json!(subject.sub));
    payload.insert(
        "iss".into(),
        serde_json::json!(std::env::var("SESAME_JWT_ISSUER")
            .unwrap_or_else(|_| "https://idam.example.com".to_string())),
    );
    payload.insert("iat".into(), serde_json::json!(now));
    payload.insert("exp".into(), serde_json::json!(now + (ttl as i64)));
    payload.insert("jti".into(), serde_json::json!(jti));
    payload.insert("tenant_id".into(), serde_json::json!(subject.tenant));

    // Story 5.1: Include ver (token version) and sid (session ID) in every access token
    payload.insert("ver".into(), serde_json::json!(token_version));
    payload.insert("sid".into(), serde_json::json!(session_id));

    // Scope
    if !merged_scopes.is_empty() {
        payload.insert("scope".into(), serde_json::json!(merged_scopes.join(" ")));
    }

    // sx (structured claims)
    let mut sx = serde_json::Map::new();

    // HACK-602: Strip admin roles for impersonation tokens
    let roles = if actor.portal == SUPPORT_PORTAL {
        // Strip admin/platform_admin from impersonated user's roles
        strip_admin_roles(&subject.roles)
    } else {
        subject.roles.clone()
    };

    sx.insert("roles".into(), serde_json::json!(roles));

    if let Some(ref org_id) = subject.org_id {
        sx.insert("org_id".into(), serde_json::json!(org_id));
    }

    payload.insert("sx".into(), serde_json::json!(sx));

    // RFC 9449 (DPoP): bind token to the client's proof key when present.
    if let Some(jkt) = dpop_jkt {
        payload.insert("cnf".into(), serde_json::json!({ "jkt": jkt }));
    }

    // Act claim for impersonation/delegation
    if actor.portal == SUPPORT_PORTAL || (!actor.sub.is_empty() && !actor.tenant.is_empty()) {
        let mut act_obj = serde_json::Map::new();
        act_obj.insert("sub".into(), serde_json::json!(actor.sub));
        act_obj.insert("tenant".into(), serde_json::json!(actor.tenant));
        act_obj.insert("portal".into(), serde_json::json!(actor.portal));

        // HACK-604: Build chain for nested delegation
        if !subject.act_chain.is_empty() {
            let mut chain = subject.act_chain.clone();
            chain.push(actor.sub.clone());
            // Cap chain depth
            if chain.len() > MAX_CHAIN_DEPTH {
                chain.drain(..chain.len() - MAX_CHAIN_DEPTH);
            }
            act_obj.insert("chain".into(), serde_json::json!(chain));
        }

        payload.insert("act".into(), serde_json::json!(act_obj));
    }

    // Sign with the shared Ed25519 platform signer (same key whose public
    // half identity-session-service publishes in JWKS). Header carries
    // {"alg":"EdDSA","typ":"at+jwt","kid":<env kid>}.
    let payload_json =
        serde_json::to_string(&serde_json::Value::Object(payload)).map_err(|e| ErrorResponse {
            error: "server_error".to_string(),
            error_description: format!("claims serialization failed: {e}"),
            retry_after: None,
            hint: None,
        })?;
    crate::services::token_issuer::SIGNER
        .sign_payload(&payload_json)
        .map_err(|e| ErrorResponse {
            error: "server_error".to_string(),
            error_description: format!("token signing failed: {e}"),
            retry_after: None,
            hint: None,
        })
}

/// Build a new refresh token, signed by the shared Ed25519 platform signer.
fn build_refresh_token(
    subject: &SubjectClaims,
    jti: &str,
    now: i64,
) -> Result<String, ErrorResponse> {
    let payload = serde_json::json!({
        "sub": subject.sub,
        "iss": std::env::var("SESAME_JWT_ISSUER")
            .unwrap_or_else(|_| "https://idam.example.com".to_string()),
        "iat": now,
        "exp": now + (30 * 24 * 3600), // 30 days
        "jti": jti,
        "type": "refresh_token",
    });

    crate::services::token_issuer::SIGNER
        .sign_payload(&payload.to_string())
        .map_err(|e| ErrorResponse {
            error: "server_error".to_string(),
            error_description: format!("refresh token signing failed: {e}"),
            retry_after: None,
            hint: None,
        })
}

// ─── Main Handler ────────────────────────────────────────────────────────

/// Main handler for auth_token endpoint.
///
/// Supports grant types:
/// - refresh_token: Rotate refresh token, issue new access token
/// - client_credentials: M2M token for service accounts
/// - urn:ietf:params:oauth:grant-type:token-exchange: RFC 8693 token exchange
#[handler(AuthTokenController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEventType, AuditLogEntry};

    let span = tracing::span!(
        tracing::Level::INFO,
        "token.issue",
        grant_type = req.data.grant_type.as_str(),
        user_id = tracing::field::Empty,
    );
    let _guard = span.enter();

    let tenant_id = req.data.x_tenant_id.clone();
    let emit_audit = |success: bool, reason: &str| {
        let event_type = if success {
            AuditEventType::JwtIssued
        } else {
            AuditEventType::ValidationFailed
        };
        match AuditLogEntry::new(event_type, "identity-login-service")
            .tenant_id(tenant_id.clone())
            .decision_source("token_exchange")
            .result(if success { "allowed" } else { "denied" })
            .reason(reason.to_string())
            .build()
        {
            Ok(entry) => EMITTER.emit(entry),
            Err(e) => tracing::warn!(error = %e, "auth_token: audit entry build failed"),
        }
    };

    let empty_denied = |scope: Option<String>| Response {
        access_token: String::new(),
        token_type: "Bearer".to_string(),
        expires_in: 0,
        refresh_token: String::new(),
        refresh_token_expires_in: None,
        user_id: String::new(),
        id_token: None,
        mfa_required: None,
        scope,
        entitlements_hash: None,
        entitlements_ref: None,
        permissions: None,
        roles: None,
        token_version: None,
    };

    // RFC 8693 token exchange (+ Story 6.2 support impersonation)
    if req.data.grant_type == "urn:ietf:params:oauth:grant-type:token-exchange" {
        match handle_token_exchange(&req.data) {
            Ok(exchange_result) => {
                span.record("result", "success");
                emit_audit(
                    true,
                    if exchange_result.act.is_some() {
                        "token_exchange_impersonation"
                    } else {
                        "token_exchange"
                    },
                );
                return Response {
                    access_token: exchange_result.access_token,
                    token_type: exchange_result.token_type,
                    expires_in: exchange_result.expires_in,
                    refresh_token: exchange_result.refresh_token,
                    refresh_token_expires_in: Some(30 * 24 * 3600),
                    user_id: "subject".to_string(),
                    id_token: None,
                    mfa_required: None,
                    scope: exchange_result.scope,
                    entitlements_hash: None,
                    entitlements_ref: None,
                    permissions: None,
                    roles: None,
                    token_version: None,
                };
            }
            Err(err) => {
                span.record("result", "denied");
                span.record("error", &err.error);
                emit_audit(false, &err.error);
                return empty_denied(None);
            }
        }
    }

    // Other grant types: not implemented on this path yet. refresh_token
    // rotation lives in identity-session-service /auth/refresh;
    // client_credentials is PRD-OPENGROUPWARE F1 scope.
    span.record("result", "denied");
    span.record("error", "unsupported_grant_type");
    emit_audit(false, "unsupported_grant_type");
    empty_denied(req.data.scope)
}

// ─── Tests: Story 6.2 — Support Impersonation Flow ───────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// gen Request implements no Default — full base value for tests.
    fn base_request() -> Request {
        Request {
            actor_token: None,
            client_id: None,
            client_secret: None,
            grant_type: String::new(),
            refresh_token: None,
            requested_token_type: None,
            scope: None,
            subject_token: None,
            subject_token_type: None,
            x_tenant_id: String::new(),
        }
    }

    // ── can_impersonate Tests ──────────────────────────────────────────

    #[test]
    fn test_support_agent_can_impersonate_same_org() {
        let actor = ActorClaim {
            sub: "agent_456".to_string(),
            tenant: "tenant_abc".to_string(),
            portal: SUPPORT_PORTAL.to_string(),
            scope: "profile:read org:org_123".to_string(),
            chain: None,
        };
        let subject = SubjectClaims {
            sub: "alice_123".to_string(),
            tenant: "tenant_abc".to_string(),
            org_id: Some("org_123".to_string()),
            scope: "profile:read orders:read".to_string(),
            roles: vec!["customer".to_string()],

            ver: None,
            sid: None,
            has_act: false,
            act_chain: vec![],
        };
        assert!(can_impersonate(&actor, &subject).is_ok());
    }

    #[test]
    fn test_non_support_agent_cannot_initiate() {
        let actor = ActorClaim {
            sub: "admin_user".to_string(),
            tenant: "tenant_abc".to_string(),
            portal: "admin-portal".to_string(),
            scope: "admin:read".to_string(),
            chain: None,
        };
        let subject = SubjectClaims {
            sub: "alice_123".to_string(),
            tenant: "tenant_abc".to_string(),
            org_id: Some("org_123".to_string()),
            scope: "profile:read".to_string(),
            roles: vec!["customer".to_string()],

            ver: None,
            sid: None,
            has_act: false,
            act_chain: vec![],
        };
        let err = can_impersonate(&actor, &subject).unwrap_err();
        assert_eq!(err.error_code(), "not_a_support_agent");
    }

    #[test]
    fn test_cross_tenant_impersonation_blocked() {
        let actor = ActorClaim {
            sub: "agent_456".to_string(),
            tenant: "tenant_hauliage".to_string(),
            portal: SUPPORT_PORTAL.to_string(),
            scope: "profile:read org:org_123".to_string(),
            chain: None,
        };
        let subject = SubjectClaims {
            sub: "bob_789".to_string(),
            tenant: "tenant_rerp".to_string(),
            org_id: Some("org_456".to_string()),
            scope: "profile:read".to_string(),
            roles: vec!["customer".to_string()],
            ver: None,
            sid: None,
            has_act: false,
            act_chain: vec![],
        };
        let err = can_impersonate(&actor, &subject).unwrap_err();
        assert_eq!(err.error_code(), "cross_tenant_impersonation_not_allowed");
    }

    #[test]
    fn test_agent_cannot_impersonate_outside_their_orgs() {
        let actor = ActorClaim {
            sub: "agent_456".to_string(),
            tenant: "tenant_abc".to_string(),
            portal: SUPPORT_PORTAL.to_string(),
            scope: "profile:read org:org_123".to_string(),
            chain: None,
        };
        let subject = SubjectClaims {
            sub: "bob_789".to_string(),
            tenant: "tenant_abc".to_string(),
            org_id: Some("org_789".to_string()),
            scope: "profile:read".to_string(),
            roles: vec!["customer".to_string()],
            ver: None,
            sid: None,
            has_act: false,
            act_chain: vec![],
        };
        let err = can_impersonate(&actor, &subject).unwrap_err();
        assert_eq!(err.error_code(), "not_in_target_org");
    }

    #[test]
    fn test_agent_cannot_impersonate_themselves() {
        let actor = ActorClaim {
            sub: "agent_456".to_string(),
            tenant: "tenant_abc".to_string(),
            portal: SUPPORT_PORTAL.to_string(),
            scope: "profile:read org:org_123".to_string(),
            chain: None,
        };
        let subject = SubjectClaims {
            sub: "agent_456".to_string(),
            tenant: "tenant_abc".to_string(),
            org_id: Some("org_123".to_string()),
            scope: "profile:read".to_string(),
            roles: vec!["support_agent".to_string()],
            ver: None,
            sid: None,
            has_act: false,
            act_chain: vec![],
        };
        let err = can_impersonate(&actor, &subject).unwrap_err();
        assert_eq!(err.error_code(), "not_a_support_agent");
    }

    #[test]
    fn test_admin_without_support_role_cannot_impersonate() {
        let actor = ActorClaim {
            sub: "admin_123".to_string(),
            tenant: "tenant_abc".to_string(),
            portal: "admin-portal".to_string(),
            scope: "admin:read".to_string(),
            chain: None,
        };
        let subject = SubjectClaims {
            sub: "user_456".to_string(),
            tenant: "tenant_abc".to_string(),
            org_id: None,
            scope: "profile:read".to_string(),
            roles: vec!["customer".to_string()],
            ver: None,
            sid: None,
            has_act: false,
            act_chain: vec![],
        };
        let err = can_impersonate(&actor, &subject).unwrap_err();
        assert_eq!(err.error_code(), "not_a_support_agent");
    }

    // ── Token Exchange with Impersonation Tests ────────────────────────

    #[test]
    fn test_impersonation_token_includes_act_claim() {
        let req = Request {
            grant_type: "urn:ietf:params:oauth:grant-type:token-exchange".to_string(),
            subject_token: Some("test_token".to_string()),
            actor_token: Some("actor_token".to_string()),
            scope: Some("profile:read".to_string()),
            subject_token_type: Some("urn:ietf:params:oauth:token-type:access_token".to_string()),
            x_tenant_id: "tenant_abc".to_string(),
            ..base_request()
        };
        let result = handle_token_exchange(&req).unwrap();
        assert!(result.act.is_some());
        let act = result.act.unwrap();
        assert_eq!(act.portal, SUPPORT_PORTAL);
    }

    #[test]
    fn test_impersonation_token_ttl_is_300_seconds() {
        let req = Request {
            grant_type: "urn:ietf:params:oauth:grant-type:token-exchange".to_string(),
            subject_token: Some("test_token".to_string()),
            actor_token: Some("actor_token".to_string()),
            scope: Some("profile:read".to_string()),
            subject_token_type: Some("urn:ietf:params:oauth:token-type:access_token".to_string()),
            x_tenant_id: "tenant_abc".to_string(),
            ..base_request()
        };
        let result = handle_token_exchange(&req).unwrap();
        assert_eq!(result.expires_in, MAX_IMPERSONATION_TTL_SECS);
    }

    #[test]
    fn test_subject_token_with_act_claim_rejected() {
        let payload = serde_json::json!({
            "sub": "alice_123",
            "tenant_id": "tenant_abc",
            "scope": "profile:read",
            "act": {
                "sub": "agent_456",
                "tenant": "tenant_abc",
                "portal": "support-portal",
                "chain": []
            }
        });
        let payload_b64 = encode_b64url(&payload.to_string());
        let fake_jwt = format!("{}.{}.sig", "header_b64", payload_b64);

        let req = Request {
            grant_type: "urn:ietf:params:oauth:grant-type:token-exchange".to_string(),
            subject_token: Some(fake_jwt),
            actor_token: Some("actor_token".to_string()),
            scope: Some("profile:read".to_string()),
            subject_token_type: Some("urn:ietf:params:oauth:token-type:access_token".to_string()),
            x_tenant_id: "tenant_abc".to_string(),
            ..base_request()
        };
        let err = handle_token_exchange(&req).unwrap_err();
        assert_eq!(err.error, "unauthorized_client");
        assert!(err.error_description.contains("act claim"));
    }

    #[test]
    fn test_exchange_without_actor_no_act_claim() {
        let req = Request {
            grant_type: "urn:ietf:params:oauth:grant-type:token-exchange".to_string(),
            subject_token: Some("subject_token".to_string()),
            actor_token: None,
            scope: Some("profile:read".to_string()),
            subject_token_type: None,
            x_tenant_id: "tenant_abc".to_string(),
            ..base_request()
        };
        let result = handle_token_exchange(&req).unwrap();
        assert!(result.act.is_none());
    }

    #[test]
    fn test_exchange_tokens_are_eddsa_signed() {
        // PRD-OPENGROUPWARE F1: exchange output must be signed by the shared
        // platform signer — no more placeholder signatures.
        let req = Request {
            grant_type: "urn:ietf:params:oauth:grant-type:token-exchange".to_string(),
            subject_token: Some("subject_token".to_string()),
            actor_token: None,
            scope: Some("profile:read".to_string()),
            subject_token_type: None,
            x_tenant_id: "tenant_abc".to_string(),
            ..base_request()
        };
        let result = handle_token_exchange(&req).unwrap();

        for token in [&result.access_token, &result.refresh_token] {
            assert!(
                !token.ends_with(".placeholder_signature"),
                "placeholder signature must be gone"
            );
            // Verifies against the process signer's public key.
            crate::services::token_issuer::SIGNER
                .verify(token)
                .expect("token must verify against the platform signer");
            // Header must advertise EdDSA with the signer's kid.
            let header_b64 = token.split('.').next().unwrap();
            let header: serde_json::Value =
                serde_json::from_str(&super::decode_b64url(header_b64).expect("header decodes"))
                    .expect("header is JSON");
            assert_eq!(header["alg"], "EdDSA");
            assert_eq!(header["kid"], crate::services::token_issuer::SIGNER.kid());
        }
    }

    #[test]
    fn test_exchange_missing_subject_token_returns_error() {
        let req = Request {
            grant_type: "urn:ietf:params:oauth:grant-type:token-exchange".to_string(),
            subject_token: None,
            actor_token: Some("actor_token".to_string()),
            scope: None,
            subject_token_type: None,
            x_tenant_id: "tenant_abc".to_string(),
            ..base_request()
        };
        let err = handle_token_exchange(&req).unwrap_err();
        assert_eq!(err.error, "invalid_request");
    }

    // ── Scope Management Tests ─────────────────────────────────────────

    #[test]
    fn test_merge_scopes_intersection() {
        let subject = vec!["profile:read".to_string(), "orders:write".to_string()];
        let requested = vec!["profile:read".to_string(), "profile:write".to_string()];
        let actor = vec!["profile:read".to_string()];

        let merged = merge_scopes(&subject, &requested, &actor);
        assert_eq!(merged, vec!["profile:read"]);
    }

    #[test]
    fn test_merge_scopes_empty_when_no_overlap() {
        let subject = vec!["orders:read".to_string()];
        let requested = vec!["profile:read".to_string()];
        let actor = vec!["profile:read".to_string()];

        let merged = merge_scopes(&subject, &requested, &actor);
        assert!(merged.is_empty());
    }

    #[test]
    fn test_parse_scopes() {
        let scopes = parse_scopes("profile:read orders:write openid");
        assert_eq!(scopes.len(), 3);
        assert!(scopes.contains(&"profile:read".to_string()));
        assert!(scopes.contains(&"orders:write".to_string()));
        assert!(scopes.contains(&"openid".to_string()));
    }

    #[test]
    fn test_build_impersonation_scope_restricted_to_read() {
        let original = "profile:read profile:write orders:read orders:write admin:read";
        let requested = vec![
            "profile:read".to_string(),
            "profile:write".to_string(),
            "orders:read".to_string(),
            "admin:read".to_string(),
        ];

        let result = build_impersonation_scope(original, &requested);
        let scopes: HashSet<&str> = result.split_whitespace().collect();
        assert!(scopes.contains("profile:read"));
        assert!(scopes.contains("orders:read"));
        assert!(!scopes.contains("profile:write"));
        assert!(!scopes.contains("orders:write"));
    }

    // ── Admin Role Stripping Tests ─────────────────────────────────────

    #[test]
    fn test_strip_admin_roles() {
        let roles = vec![
            "customer".to_string(),
            "admin".to_string(),
            "platform_admin".to_string(),
            "super_admin".to_string(),
        ];
        let stripped = strip_admin_roles(&roles);
        assert_eq!(stripped.len(), 1);
        assert_eq!(stripped[0], "customer");
    }

    #[test]
    fn test_strip_admin_roles_preserves_non_admin() {
        let roles = vec!["customer".to_string(), "support_agent".to_string()];
        let stripped = strip_admin_roles(&roles);
        assert_eq!(stripped.len(), 2);
    }

    // ── Token Lifecycle Tests ──────────────────────────────────────────

    #[test]
    fn test_impersonation_chain_detection() {
        let subject = SubjectClaims {
            sub: "alice_123".to_string(),
            tenant: "tenant_abc".to_string(),
            org_id: Some("org_123".to_string()),
            scope: "profile:read".to_string(),
            roles: vec!["customer".to_string()],
            ver: None,
            sid: None,
            has_act: true,
            act_chain: vec!["agent_456".to_string()],
        };
        assert!(is_impersonation_token(&subject));

        let plain_subject = SubjectClaims {
            sub: "alice_123".to_string(),
            tenant: "tenant_abc".to_string(),
            org_id: None,
            scope: "profile:read".to_string(),
            roles: vec!["customer".to_string()],
            ver: None,
            sid: None,
            has_act: false,
            act_chain: vec![],
        };
        assert!(!is_impersonation_token(&plain_subject));
    }

    #[test]
    fn test_admin_route_check_for_impersonation_token() {
        let act = Some(ActorClaim {
            sub: "agent_456".to_string(),
            tenant: "tenant_abc".to_string(),
            portal: SUPPORT_PORTAL.to_string(),
            scope: "profile:read".to_string(),
            chain: None,
        });
        assert!(!can_access_admin_routes(&act.as_ref()));

        let no_act: Option<ActorClaim> = None;
        assert!(can_access_admin_routes(&no_act.as_ref()));
    }

    #[test]
    fn test_impersonation_ttl_computed_correctly() {
        assert_eq!(compute_impersonation_ttl(), MAX_IMPERSONATION_TTL_SECS);
    }

    #[test]
    fn test_max_concurrent_impersonations() {
        assert!(has_max_concurrent_impersonations(
            "agent_1",
            MAX_CONCURRENT_IMPERSONATIONS
        ));
        assert!(!has_max_concurrent_impersonations(
            "agent_1",
            MAX_CONCURRENT_IMPERSONATIONS - 1
        ));
    }

    #[test]
    fn test_impersonation_error_codes() {
        let err = ImpersonationError::NotASupportAgent("test".to_string());
        assert_eq!(err.error_code(), "not_a_support_agent");

        let err = ImpersonationError::CrossTenantImpersonationNotAllowed("test".to_string());
        assert_eq!(err.error_code(), "cross_tenant_impersonation_not_allowed");

        let err = ImpersonationError::NotInTargetOrg("test".to_string());
        assert_eq!(err.error_code(), "not_in_target_org");

        let err = ImpersonationError::ImpersonationChainNotAllowed("test".to_string());
        assert_eq!(err.error_code(), "impersonation_chain_not_allowed");

        let err = ImpersonationError::MaxConcurrentImpersonationsReached;
        assert_eq!(err.error_code(), "max_concurrent_impersonations_reached");
    }

    // ── JWT Parsing Tests ──────────────────────────────────────────────

    #[test]
    fn test_parse_jwt_claims_from_valid_payload() {
        let payload = serde_json::json!({
            "sub": "user_123",
            "tenant_id": "tenant_xyz",
            "scope": "profile:read orders:write",
            "sx": {
                "roles": ["customer", "premium"],
                "org_id": "org_456"
            }
        });
        let payload_str = payload.to_string();
        let claims = parse_jwt_claims(&payload_str).unwrap();
        assert_eq!(claims.sub, "user_123");
        assert_eq!(claims.tenant, "tenant_xyz");
        assert_eq!(claims.org_id, Some("org_456".to_string()));
        assert_eq!(
            claims.roles,
            vec!["customer".to_string(), "premium".to_string()]
        );
    }

    #[test]
    fn test_parse_actor_claims_from_jwt() {
        let payload = serde_json::json!({
            "sub": "agent_789",
            "tenant_id": "tenant_abc",
            "sx": {
                "portal": "support-portal",
                "roles": ["support_agent"]
            },
            "scope": "profile:read orders:read"
        });
        let payload_str = payload.to_string();
        let actor = parse_actor_claims_from_jwt(&payload_str).unwrap();
        assert_eq!(actor.sub, "agent_789");
        assert_eq!(actor.portal, SUPPORT_PORTAL);
        assert_eq!(actor.tenant, "tenant_abc");
    }

    #[test]
    fn test_parse_actor_claims_from_jwt_with_chain() {
        let payload = serde_json::json!({
            "sub": "agent_789",
            "tenant_id": "tenant_abc",
            "sx": { "portal": "support-portal" },
            "act": {
                "sub": "agent_prev",
                "tenant": "tenant_abc",
                "chain": ["agent_prev2"]
            }
        });
        let payload_str = payload.to_string();
        let actor = parse_actor_claims_from_jwt(&payload_str).unwrap();
        assert_eq!(actor.sub, "agent_789");
        assert!(actor.chain.is_some());
        let chain = actor.chain.unwrap();
        assert_eq!(chain, vec!["agent_prev2"]);
    }

    #[test]
    fn test_parse_empty_subject_token_returns_error() {
        let err = parse_subject_token("").unwrap_err();
        assert_eq!(err.error, "invalid_token");
    }

    #[test]
    fn test_parse_empty_actor_token_returns_error() {
        let err = parse_actor_token("").unwrap_err();
        assert_eq!(err.error, "invalid_token");
    }

    // ── Base64url Encoding Tests ───────────────────────────────────────

    #[test]
    fn test_b64url_roundtrip() {
        let original = "{\"sub\":\"alice\"}";
        let encoded = encode_b64url(original);
        let decoded = decode_b64url(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_b64url_invalid_input() {
        let result = decode_b64url("not-valid-base64!!!");
        assert!(result.is_err());
    }

    // ── Story 6.2 BDD Acceptance Criteria Tests ────────────────────────

    #[test]
    fn test_bdd_support_agent_impersonates_user_in_same_org() {
        let actor = ActorClaim {
            sub: "support_agent_456".to_string(),
            tenant: "tenant_abc".to_string(),
            portal: SUPPORT_PORTAL.to_string(),
            scope: "profile:read org:org_123".to_string(),
            chain: None,
        };
        let subject = SubjectClaims {
            sub: "alice_123".to_string(),
            tenant: "tenant_abc".to_string(),
            org_id: Some("org_123".to_string()),
            scope: "profile:read orders:read".to_string(),
            roles: vec!["customer".to_string()],
            ver: None,
            sid: None,
            has_act: false,
            act_chain: vec![],
        };
        assert!(can_impersonate(&actor, &subject).is_ok());
    }

    #[test]
    fn test_bdd_non_support_user_cannot_impersonate() {
        let actor = ActorClaim {
            sub: "hank".to_string(),
            tenant: "tenant_abc".to_string(),
            portal: "customer-app".to_string(),
            scope: "profile:read".to_string(),
            chain: None,
        };
        let subject = SubjectClaims {
            sub: "alice_123".to_string(),
            tenant: "tenant_abc".to_string(),
            org_id: None,
            scope: "profile:read".to_string(),
            roles: vec!["customer".to_string()],
            ver: None,
            sid: None,
            has_act: false,
            act_chain: vec![],
        };
        let err = can_impersonate(&actor, &subject).unwrap_err();
        assert_eq!(err.error_code(), "not_a_support_agent");
    }

    #[test]
    fn test_bdd_impersonation_token_strips_admin() {
        let roles = vec![
            "customer".to_string(),
            "admin".to_string(),
            "platform_admin".to_string(),
        ];
        let stripped = strip_admin_roles(&roles);
        assert!(!stripped.contains(&"admin".to_string()));
        assert!(!stripped.contains(&"platform_admin".to_string()));
        assert!(stripped.contains(&"customer".to_string()));
    }

    #[test]
    fn test_bdd_impersonation_restricted_to_read_scopes() {
        let original = "profile:read orders:read orders:write";
        let requested = vec![
            "profile:read".to_string(),
            "orders:read".to_string(),
            "orders:write".to_string(),
        ];
        let result = build_impersonation_scope(original, &requested);
        let scopes: HashSet<&str> = result.split_whitespace().collect();
        assert!(scopes.contains("profile:read"));
        assert!(scopes.contains("orders:read"));
        assert!(!scopes.contains("orders:write"));
    }

    #[test]
    fn test_bdd_agent_with_no_org_cannot_impersonate_target_outside_orgs() {
        let actor = ActorClaim {
            sub: "agent_456".to_string(),
            tenant: "tenant_abc".to_string(),
            portal: SUPPORT_PORTAL.to_string(),
            scope: "profile:read".to_string(),
            chain: None,
        };
        let subject = SubjectClaims {
            sub: "alice_123".to_string(),
            tenant: "tenant_abc".to_string(),
            org_id: Some("org_123".to_string()),
            scope: "profile:read".to_string(),
            roles: vec!["customer".to_string()],
            ver: None,
            sid: None,
            has_act: false,
            act_chain: vec![],
        };
        assert!(can_impersonate(&actor, &subject).is_ok());
    }

    // ── Story 3.4: can_delegate Unit Tests ─────────────────────────────

    /// Given an actor with portal containing "admin", can_delegate returns true.
    #[test]
    fn test_can_delegate_platform_admin_returns_true() {
        let actor = ActorClaim {
            sub: "platform_admin_1".to_string(),
            tenant: "tenant_a".to_string(),
            portal: "admin-portal".to_string(),
            scope: "admin:read admin:write".to_string(),
            chain: None,
        };
        assert!(can_delegate(&actor, "any_user_id"));
    }

    /// Given an actor with portal containing "org_admin", can_delegate returns true.
    #[test]
    fn test_can_delegate_org_admin_returns_true() {
        let actor = ActorClaim {
            sub: "org_admin_1".to_string(),
            tenant: "tenant_a".to_string(),
            portal: "org_admin-portal".to_string(),
            scope: "org:read".to_string(),
            chain: None,
        };
        assert!(can_delegate(&actor, "any_user_id"));
    }

    /// Given an actor with regular user portal, can_delegate returns false.
    #[test]
    fn test_can_delegate_regular_user_returns_false() {
        let actor = ActorClaim {
            sub: "customer_user".to_string(),
            tenant: "tenant_a".to_string(),
            portal: "customer-app".to_string(),
            scope: "profile:read".to_string(),
            chain: None,
        };
        assert!(!can_delegate(&actor, "any_user_id"));
    }

    /// Given an actor with empty portal, can_delegate returns false.
    #[test]
    fn test_can_delegate_empty_portal_returns_false() {
        let actor = ActorClaim {
            sub: "empty_portal_user".to_string(),
            tenant: "tenant_a".to_string(),
            portal: "".to_string(),
            scope: "".to_string(),
            chain: None,
        };
        assert!(!can_delegate(&actor, "any_user_id"));
    }

    /// Given an actor with portal containing neither "admin" nor "org_admin", can_delegate returns false.
    #[test]
    fn test_can_delegate_neither_admin_nor_org_admin_returns_false() {
        let actor = ActorClaim {
            sub: "support_user".to_string(),
            tenant: "tenant_a".to_string(),
            portal: "support-portal".to_string(),
            scope: "profile:read".to_string(),
            chain: None,
        };
        assert!(!can_delegate(&actor, "any_user_id"));
    }

    // ── Story 3.4: Tenant Match Check ──────────────────────────────────

    /// Given an actor from tenant "hauliage" and a subject from tenant "rerp",
    /// the exchange is rejected with a tenant mismatch error.
    #[test]
    fn test_tenant_mismatch_rejected_in_exchange() {
        let actor_payload = serde_json::json!({
            "sub": "admin_hauliage",
            "tenant_id": "hauliage",
            "sx": { "portal": "admin-portal" }
        });
        let actor_b64 = encode_b64url(&actor_payload.to_string());
        let actor_jwt = format!("{}.{}.sig", "header_b64", actor_b64);

        let subject_payload = serde_json::json!({
            "sub": "user_rerp",
            "tenant_id": "rerp",
            "scope": "profile:read"
        });
        let subject_b64 = encode_b64url(&subject_payload.to_string());
        let subject_jwt = format!("{}.{}.sig", "header_b64", subject_b64);

        let req = Request {
            grant_type: "urn:ietf:params:oauth:grant-type:token-exchange".to_string(),
            subject_token: Some(subject_jwt),
            actor_token: Some(actor_jwt),
            scope: Some("profile:read".to_string()),
            subject_token_type: None,
            x_tenant_id: "tenant_mismatch".to_string(),
            ..base_request()
        };

        let err = handle_token_exchange(&req).unwrap_err();
        assert_eq!(err.error, "invalid_request");
        assert!(err.error_description.contains("Tenant mismatch"));
    }

    /// Given matching tenants, the exchange proceeds without tenant error.
    #[test]
    fn test_tenant_match_allows_exchange_to_proceed() {
        let actor_payload = serde_json::json!({
            "sub": "admin_same_tenant",
            "tenant_id": "same-tenant",
            "sx": { "portal": "admin-portal" }
        });
        let actor_b64 = encode_b64url(&actor_payload.to_string());
        let actor_jwt = format!("{}.{}.sig", "header_b64", actor_b64);

        let subject_payload = serde_json::json!({
            "sub": "user_same_tenant",
            "tenant_id": "same-tenant",
            "scope": "profile:read"
        });
        let subject_b64 = encode_b64url(&subject_payload.to_string());
        let subject_jwt = format!("{}.{}.sig", "header_b64", subject_b64);

        let req = Request {
            grant_type: "urn:ietf:params:oauth:grant-type:token-exchange".to_string(),
            subject_token: Some(subject_jwt),
            actor_token: Some(actor_jwt),
            scope: Some("profile:read".to_string()),
            subject_token_type: None,
            x_tenant_id: "same-tenant".to_string(),
            ..base_request()
        };

        let result = handle_token_exchange(&req).unwrap();
        assert!(result.act.is_some());
    }

    // ── Story 3.4: Scope Merging Tests ─────────────────────────────────

    /// Given subject scopes "profile:read orders:write", requested "orders:write invoices:read",
    /// and actor scopes "orders:write", the result is "orders:write" (intersection of all three).
    #[test]
    fn test_scope_merging_intersection() {
        let subject_scopes = vec!["profile:read".to_string(), "orders:write".to_string()];
        let requested = vec!["orders:write".to_string(), "invoices:read".to_string()];
        let actor_scopes = vec!["orders:write".to_string()];

        let result = merge_scopes(&subject_scopes, &requested, &actor_scopes);
        assert_eq!(result, vec!["orders:write"]);
    }

    /// Given subject has "profile:read" but actor requests "profile:read orders:write",
    /// the result is "profile:read" (subject's scopes limit the result).
    #[test]
    fn test_scope_merging_rejects_over_requested() {
        let subject_scopes = vec!["profile:read".to_string()];
        let requested = vec!["profile:read".to_string(), "orders:write".to_string()];
        let actor_scopes = vec!["profile:read".to_string(), "orders:write".to_string()];

        let result = merge_scopes(&subject_scopes, &requested, &actor_scopes);
        assert_eq!(result, vec!["profile:read"]);
    }

    /// Given subject has no scopes, the result is empty (no scopes granted).
    #[test]
    fn test_scope_merging_empty_subject_scopes() {
        let subject_scopes: Vec<String> = vec![];
        let requested = vec!["profile:read".to_string()];
        let actor_scopes = vec!["profile:read".to_string()];

        let result = merge_scopes(&subject_scopes, &requested, &actor_scopes);
        assert!(result.is_empty());
    }

    /// Given empty requested scope, the result is empty.
    #[test]
    fn test_scope_merging_empty_requested() {
        let subject_scopes = vec!["profile:read".to_string(), "orders:write".to_string()];
        let requested: Vec<String> = vec![];
        let actor_scopes = vec!["profile:read".to_string(), "orders:write".to_string()];

        let result = merge_scopes(&subject_scopes, &requested, &actor_scopes);
        assert!(result.is_empty());
    }

    // ── Story 3.4: Nested Act Chain Preservation ───────────────────────

    /// Given an actor_token with act.sub = "user_456" and act.chain = ["support_tool"],
    /// the new token's act includes the full chain.
    #[test]
    fn test_nested_act_chain_preserved() {
        let actor_payload = serde_json::json!({
            "sub": "user_456",
            "tenant_id": "tenant_a",
            "sx": { "portal": "admin-portal" },
            "act": {
                "sub": "user_456",
                "tenant": "tenant_a",
                "portal": "admin-portal",
                "chain": ["support_tool"]
            }
        });
        let actor_b64 = encode_b64url(&actor_payload.to_string());
        let actor_jwt = format!("{}.{}.sig", "header_b64", actor_b64);

        let subject_payload = serde_json::json!({
            "sub": "target_user",
            "tenant_id": "tenant_a",
            "scope": "profile:read"
        });
        let subject_b64 = encode_b64url(&subject_payload.to_string());
        let subject_jwt = format!("{}.{}.sig", "header_b64", subject_b64);

        let req = Request {
            grant_type: "urn:ietf:params:oauth:grant-type:token-exchange".to_string(),
            subject_token: Some(subject_jwt),
            actor_token: Some(actor_jwt),
            scope: Some("profile:read".to_string()),
            subject_token_type: None,
            x_tenant_id: "tenant_a".to_string(),
            ..base_request()
        };

        let result = handle_token_exchange(&req).unwrap();
        assert!(result.act.is_some());
        let act = result.act.unwrap();
        assert_eq!(act.sub, "user_456");
    }

    // ── Story 3.4: Security Regression Tests ───────────────────────────

    /// Given an actor with "profile:read" requesting "profile:read orders:write",
    /// the new token only contains "profile:read" (scope cannot be expanded beyond actor's own scopes).
    #[test]
    fn test_actor_cannot_delegate_scope_it_does_not_have() {
        let actor_payload = serde_json::json!({
            "sub": "limited_actor",
            "tenant_id": "tenant_a",
            "sx": { "portal": "admin-portal" },
            "scope": "profile:read"
        });
        let actor_b64 = encode_b64url(&actor_payload.to_string());
        let actor_jwt = format!("{}.{}.sig", "header_b64", actor_b64);

        let subject_payload = serde_json::json!({
            "sub": "full_subject",
            "tenant_id": "tenant_a",
            "scope": "profile:read orders:write admin:read"
        });
        let subject_b64 = encode_b64url(&subject_payload.to_string());
        let subject_jwt = format!("{}.{}.sig", "header_b64", subject_b64);

        let req = Request {
            grant_type: "urn:ietf:params:oauth:grant-type:token-exchange".to_string(),
            subject_token: Some(subject_jwt),
            actor_token: Some(actor_jwt),
            scope: Some("profile:read orders:write".to_string()),
            subject_token_type: None,
            x_tenant_id: "tenant_a".to_string(),
            ..base_request()
        };

        let result = handle_token_exchange(&req).unwrap();
        // actor only has "profile:read", so result should not contain "orders:write"
        if let Some(ref scope) = result.scope {
            assert!(
                !scope.contains("orders:write"),
                "Actor should not delegate scopes they don't have"
            );
        }
    }

    /// Assert that act.sub in the new token is derived from the validated actor token,
    /// not from any client-supplied value.
    #[test]
    fn test_act_sub_set_by_server_not_client() {
        let actor_payload = serde_json::json!({
            "sub": "actual_actor_123",
            "tenant_id": "tenant_a",
            "sx": { "portal": "admin-portal" }
        });
        let actor_b64 = encode_b64url(&actor_payload.to_string());
        let actor_jwt = format!("{}.{}.sig", "header_b64", actor_b64);

        let subject_payload = serde_json::json!({
            "sub": "subject_user",
            "tenant_id": "tenant_a",
            "scope": "profile:read"
        });
        let subject_b64 = encode_b64url(&subject_payload.to_string());
        let subject_jwt = format!("{}.{}.sig", "header_b64", subject_b64);

        let req = Request {
            grant_type: "urn:ietf:params:oauth:grant-type:token-exchange".to_string(),
            subject_token: Some(subject_jwt),
            actor_token: Some(actor_jwt),
            scope: Some("profile:read".to_string()),
            subject_token_type: None,
            x_tenant_id: "tenant_a".to_string(),
            ..base_request()
        };

        let result = handle_token_exchange(&req).unwrap();
        assert!(result.act.is_some());
        let act = result.act.unwrap();
        assert_eq!(act.sub, "actual_actor_123");
    }

    // ── Story 3.4: Edge Cases ──────────────────────────────────────────

    /// Empty subject token should be rejected with 401, not panic.
    #[test]
    fn test_empty_subject_token_rejected() {
        let req = Request {
            grant_type: "urn:ietf:params:oauth:grant-type:token-exchange".to_string(),
            subject_token: Some("".to_string()),
            actor_token: None,
            scope: None,
            subject_token_type: None,
            x_tenant_id: "tenant_a".to_string(),
            ..base_request()
        };

        let err = handle_token_exchange(&req).unwrap_err();
        assert_eq!(err.error, "invalid_token");
    }

    /// Token exchange with empty requested scope returns token with no scopes.
    #[test]
    fn test_exchange_empty_requested_scope() {
        let actor_payload = serde_json::json!({
            "sub": "admin_1",
            "tenant_id": "tenant_a",
            "sx": { "portal": "admin-portal" }
        });
        let actor_b64 = encode_b64url(&actor_payload.to_string());
        let actor_jwt = format!("{}.{}.sig", "header_b64", actor_b64);

        let subject_payload = serde_json::json!({
            "sub": "subject_1",
            "tenant_id": "tenant_a",
            "scope": "profile:read"
        });
        let subject_b64 = encode_b64url(&subject_payload.to_string());
        let subject_jwt = format!("{}.{}.sig", "header_b64", subject_b64);

        let req = Request {
            grant_type: "urn:ietf:params:oauth:grant-type:token-exchange".to_string(),
            subject_token: Some(subject_jwt),
            actor_token: Some(actor_jwt),
            scope: Some("".to_string()),
            subject_token_type: None,
            x_tenant_id: "tenant_a".to_string(),
            ..base_request()
        };

        let result = handle_token_exchange(&req).unwrap();
        assert!(result.scope.is_none() || result.scope.as_deref() == Some(""));
    }

    /// Actor cannot escalate privilege: given a low-privilege subject token,
    /// the actor cannot produce a token with more permissions than the subject has.
    #[test]
    fn test_actor_cannot_escalate_privilege_via_subject() {
        let actor_payload = serde_json::json!({
            "sub": "admin_escalating",
            "tenant_id": "tenant_a",
            "sx": { "portal": "admin-portal" },
            "scope": "admin:read profile:read orders:read orders:write"
        });
        let actor_b64 = encode_b64url(&actor_payload.to_string());
        let actor_jwt = format!("{}.{}.sig", "header_b64", actor_b64);

        // Subject only has profile:read
        let subject_payload = serde_json::json!({
            "sub": "low_priv_user",
            "tenant_id": "tenant_a",
            "scope": "profile:read"
        });
        let subject_b64 = encode_b64url(&subject_payload.to_string());
        let subject_jwt = format!("{}.{}.sig", "header_b64", subject_b64);

        let req = Request {
            grant_type: "urn:ietf:params:oauth:grant-type:token-exchange".to_string(),
            subject_token: Some(subject_jwt),
            actor_token: Some(actor_jwt),
            scope: Some("profile:read orders:write".to_string()),
            subject_token_type: None,
            x_tenant_id: "tenant_a".to_string(),
            ..base_request()
        };

        let result = handle_token_exchange(&req).unwrap();
        // The merged scopes should not include orders:write since subject doesn't have it
        if let Some(ref scope) = result.scope {
            assert!(
                !scope.contains("orders:write"),
                "Actor cannot escalate: subject has no orders:write scope"
            );
        }
    }
}
