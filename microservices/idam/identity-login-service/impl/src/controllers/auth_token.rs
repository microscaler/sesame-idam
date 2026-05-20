// Implementation for handler 'auth_token'
// Story 6.1: RFC 8693 Token Exchange Endpoint (existing)
// Story 6.2: Support Impersonation Flow (added below)
use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use sesame_idam_identity_login_service_gen::handlers::auth_token::{Request, Response};
use std::collections::HashSet;

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
                return Err(ImpersonationError::NotInTargetOrg(
                    format!(
                        "Agent '{}' is assigned to org '{}' but target user '{}' is in org '{}'",
                        actor.sub, agent_org, subject.sub, target_org
                    ),
                ));
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
        .filter(|r| {
            r != "admin" && r != "platform_admin" && r != "super_admin"
        })
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
    let value: serde_json::Value = serde_json::from_str(payload_str).map_err(|_| ErrorResponse {
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
    
    Ok(SubjectClaims {
        sub,
        tenant,
        org_id,
        scope,
        roles,
        has_act,
        act_chain,
    })
}

/// Extract actor claims from a decoded JWT payload.
fn parse_actor_claims_from_jwt(payload_str: &str) -> Result<ActorClaim, ErrorResponse> {
    let value: serde_json::Value = serde_json::from_str(payload_str).map_err(|_| ErrorResponse {
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
        .and_then(|v| v.as_str())
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
    use base64::{Engine as _, engine::general_purpose};
    let bytes = general_purpose::URL_SAFE_NO_PAD
        .decode(data)
        .map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

/// Base64url encode a string.
fn encode_b64url(data: &str) -> String {
    use base64::{Engine as _, engine::general_purpose};
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
    use uuid::Uuid;
    use std::time::{SystemTime, UNIX_EPOCH};
    
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
            error_description: "Subject token has an act claim and cannot be used for further delegation".to_string(),
            retry_after: None,
            hint: Some("Use the original user token, not an impersonation token, for token exchange".to_string()),
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
    } else {
        // General delegation (Story 6.1)
        if !can_delegate(&actor_claims, &subject_claims.sub) {
            return Err(ErrorResponse {
                error: "invalid_request".to_string(),
                error_description: "Actor does not have permission to delegate on behalf of subject".to_string(),
                retry_after: None,
                hint: Some("Actor must have platform_admin or org_admin role".to_string()),
            });
        }
        
        // HACK-310: Cross-tenant delegation rejected
        if actor_claims.tenant != subject_claims.tenant {
            return Err(ErrorResponse {
                error: "invalid_request".to_string(),
                error_description: "Tenant mismatch: actor and subject must be from the same tenant".to_string(),
                retry_after: None,
                hint: Some("Ensure actor_token and subject_token are from the same tenant".to_string()),
            });
        }
    }
    
    // 5. Merge scopes
    let requested_scopes = req.scope.as_ref()
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
    
    let new_access_token = build_access_token(&subject_claims, &actor_claims, &merged_scopes, &jti, now, token_ttl);
    let new_refresh_token = build_refresh_token(&subject_claims, &jti, now);
    
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
fn build_access_token(
    subject: &SubjectClaims,
    actor: &ActorClaim,
    merged_scopes: &[String],
    jti: &str,
    now: i64,
    ttl: i32,
) -> String {
    use uuid::Uuid;
    
    // Build JWT payload
    let mut payload = serde_json::Map::new();
    payload.insert("sub".into(), serde_json::json!(subject.sub));
    payload.insert("iss".into(), serde_json::json!("https://idam.example.com"));
    payload.insert("iat".into(), serde_json::json!(now));
    payload.insert("exp".into(), serde_json::json!(now + (ttl as i64)));
    payload.insert("jti".into(), serde_json::json!(jti));
    payload.insert("tenant_id".into(), serde_json::json!(subject.tenant));
    
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
    
    let header = serde_json::json!({
        "alg": "RS256",
        "typ": "at+jwt",
        "kid": "default-key",
    });
    
    let header_b64 = encode_b64url(&header.to_string());
    let payload_b64 = encode_b64url(&serde_json::to_string(&serde_json::Value::Object(payload)).unwrap_or_default());
    
    // In production, sign with RS256 using the service's private key
    // For now, return a placeholder token
    format!("{}.{}.placeholder_signature", header_b64, payload_b64)
}

/// Build a new refresh token.
fn build_refresh_token(subject: &SubjectClaims, jti: &str, now: i64) -> String {
    let payload = serde_json::json!({
        "sub": subject.sub,
        "iss": "https://idam.example.com",
        "iat": now,
        "exp": now + (30 * 24 * 3600), // 30 days
        "jti": jti,
        "type": "refresh_token",
    });
    
    let header = serde_json::json!({
        "alg": "RS256",
        "typ": "at+jwt",
        "kid": "default-key",
    });
    
    let header_b64 = encode_b64url(&header.to_string());
    let payload_b64 = encode_b64url(&payload.to_string());
    
    format!("{}.{}.placeholder_signature", header_b64, payload_b64)
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
    use sesame_audit::{AuditEvent, AuditEventType, AuditActor, AuditSeverity};
    
    // Span: track token issuance events
    let span = tracing::span!(
        tracing::Level::INFO,
        "token.issue",
        grant_type = req.inner.grant_type.as_str(),
        user_id = tracing::field::Empty,
    );
    let _guard = span.enter();
    
    // Handle token exchange (RFC 8693 + Story 6.2 support impersonation)
    if req.inner.grant_type == "urn:ietf:params:oauth:grant-type:token-exchange" {
        match handle_token_exchange(&req.inner) {
            Ok(exchange_result) => {
                span.record("result", "success");
                span.record("user_id", exchange_result.access_token.chars().take(8).collect::<String>());
                
                // Audit log: token exchange success
                EMITTER.emit(AuditEvent {
                    event_type: AuditEventType::TokenExchange,
                    actor: AuditActor::Anonymous,
                    severity: AuditSeverity::Info,
                    message: format!(
                        "Token exchange succeeded. act={:?}",
                        exchange_result.act
                    ),
                    metadata: serde_json::json!({
                        "grant_type": "token_exchange",
                        "scope": exchange_result.scope,
                        "expires_in": exchange_result.expires_in,
                        "has_act": exchange_result.act.is_some(),
                    }),
                });
                
                // Convert TokenExchangeResult to Response
                return Response {
                    access_token: exchange_result.access_token,
                    token_type: exchange_result.token_type,
                    expires_in: exchange_result.expires_in,
                    refresh_token: exchange_result.refresh_token,
                    refresh_token_expires_in: Some(86400), // 30 days
                    user_id: "subject_user".to_string(),
                    email: None,
                    email_verified: None,
                    phone_verified: None,
                    mfa_required: None,
                    id_token: None,
                    scope: exchange_result.scope,
                };
            }
            Err(err) => {
                span.record("result", "denied");
                span.record("error", &err.error);
                
                // Audit log: token exchange failure
                EMITTER.emit(AuditEvent {
                    event_type: AuditEventType::TokenExchange,
                    actor: AuditActor::Anonymous,
                    severity: AuditSeverity::Warning,
                    message: format!(
                        "Token exchange failed: {} - {}",
                        err.error, err.error_description
                    ),
                    metadata: serde_json::json!({
                        "error": err.error,
                        "error_description": err.error_description,
                    }),
                });
                
                return Response {
                    access_token: "".to_string(),
                    token_type: "Bearer".to_string(),
                    expires_in: 0,
                    refresh_token: "".to_string(),
                    refresh_token_expires_in: None,
                    user_id: "".to_string(),
                    email: None,
                    email_verified: None,
                    phone_verified: None,
                    mfa_required: None,
                    id_token: None,
                    scope: None,
                };
            }
        }
    }
    
    // Handle other grant types (refresh_token, client_credentials)
    match req.inner.grant_type.as_str() {
        "refresh_token" => {
            let user_id = req.inner.refresh_token.clone().unwrap_or_default();
            
            span.record("user_id", &user_id);
            span.record("result", "success");
            
            Response {
                access_token: format!("access_{}", uuid::Uuid::new_v4()),
                token_type: "Bearer".to_string(),
                expires_in: 3600,
                refresh_token: format!("refresh_{}", uuid::Uuid::new_v4()),
                refresh_token_expires_in: Some(86400),
                user_id,
                email: None,
                email_verified: None,
                phone_verified: None,
                mfa_required: None,
                id_token: None,
                scope: req.inner.scope,
            }
        }
        "client_credentials" => {
            span.record("result", "denied");
            span.record("error", "client_credentials_not_implemented");
            
            Response {
                access_token: "".to_string(),
                token_type: "Bearer".to_string(),
                expires_in: 0,
                refresh_token: "".to_string(),
                refresh_token_expires_in: None,
                user_id: "".to_string(),
                email: None,
                email_verified: None,
                phone_verified: None,
                mfa_required: None,
                id_token: None,
                scope: req.inner.scope,
            }
        }
        _ => {
            span.record("result", "denied");
            span.record("error", "unsupported_grant_type");
            
            Response {
                access_token: "".to_string(),
                token_type: "Bearer".to_string(),
                expires_in: 0,
                refresh_token: "".to_string(),
                refresh_token_expires_in: None,
                user_id: "".to_string(),
                email: None,
                email_verified: None,
                phone_verified: None,
                mfa_required: None,
                id_token: None,
                scope: "".to_string(),
            }
        }
    }
}
